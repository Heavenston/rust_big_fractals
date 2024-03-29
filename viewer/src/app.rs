
use std::{
    borrow::Cow,
    time::{Duration, Instant},
    collections::HashSet,
    sync::{Arc, Mutex, atomic::AtomicBool}, path::Path, future::Future
};

use glyph_brush::{ab_glyph::FontRef, OwnedText};
use image::EncodableLayout;
use wgpu::util::DeviceExt;
use winit::{
    event::{Event, WindowEvent, ElementState, VirtualKeyCode, MouseScrollDelta},
    event_loop::{EventLoop, ControlFlow},
    window::{WindowBuilder, Window},
};
use itertools::Itertools;

use crate::format::FormattedBigImage;

const IMAGE_SECTION_SIZE: u32 = 2048;

pub async fn start_app(folder: &Path) {
    BigImageApp::new(FormattedBigImage::load_folder(folder).await).await
}

#[derive(Debug, Clone, Copy)]
struct SectionPosition {
    pub subdivisions: u32,
    pub pos: (u32, u32),
}

#[derive(Debug)]
struct ImageSection {
    pub loading_image: Arc<(AtomicBool, Mutex<Option<image::RgbaImage>>)>,
    pub hide: bool,

    pub image: image::RgbaImage,
    pub bind_group: wgpu::BindGroup,
    pub texture: wgpu::Texture,
    pub sampler: wgpu::Sampler,
    pub transform_buffer: wgpu::Buffer,

    pub position: SectionPosition,
}

struct BigImageApp {
    image: Arc<FormattedBigImage>,

    window: Window,

    surface_config: wgpu::SurfaceConfiguration,

    wgpu_instance: wgpu::Instance,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,

    device: wgpu::Device,
    queue: wgpu::Queue,

    render_pipeline: wgpu::RenderPipeline,
    brush: wgpu_text::TextBrush<FontRef<'static>>,
    debug_text: wgpu_text::section::OwnedSection,

    viewport_bind_group_layout: wgpu::BindGroupLayout,
    viewport_bind_group: wgpu::BindGroup,
    viewport_transform_buffer: wgpu::Buffer,

    image_section_bind_group_layout: wgpu::BindGroupLayout,
    image_sections: Vec<ImageSection>,

    camera_x: f32,
    camera_y: f32,
    camera_zoom: f32,

    pressed_keys: HashSet<VirtualKeyCode>,
    just_pressed_keys: HashSet<VirtualKeyCode>,
}

impl BigImageApp {
    pub async fn new(image: FormattedBigImage) {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop).unwrap();

        let wgpu_instance = wgpu::Instance::default();
        let surface = unsafe { wgpu_instance.create_surface(&window) }
            .expect("Could not create surface");
        let adapter = wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),

                ..wgpu::RequestAdapterOptionsBase::default()
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adapter.limits()),
            }, None).await.expect("Failed to create device");

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                Cow::Borrowed(include_str!("shader.wgsl"))
            ),
        });

        let viewport_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None
                },
                count: None,
            }],
        });

        let viewport_transform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of::<[f32; 12]>(&[
                1., 0., 0.,    /* PADDING */ 0.,
                0., 1., 0.,    /* PADDING */ 0.,
                0., 0., 1.,    /* PADDING */ 0.,
            ]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let viewport_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &viewport_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        viewport_transform_buffer.as_entire_buffer_binding()
                    ),
                }
            ],
        });

        let image_section_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false
                },
                count: None,
            },wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&viewport_bind_group_layout, &image_section_bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "vertex_main",
                    buffers: &[]
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "fragment_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: swapchain_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::all(),
                    })],
                }),
                multiview: None,
            });

        let font = include_bytes!("Inter-VariableFont_slnt,wght.ttf");
        let brush = wgpu_text::BrushBuilder::using_font_bytes(font)
            .unwrap().build(
                &device,
                &surface_config
            );

        use wgpu_text::section::*;
        let debug_text = Section::default()
            .add_text(
                Text::new("Rendering: 0")
                .with_scale(50.)
                .with_color([0., 0., 0., 2.]),
            )
            .with_bounds((surface_config.width as f32 * 0.4, surface_config.height as f32))
            .with_layout(
                Layout::default()
                    .v_align(VerticalAlign::Top)
                    .h_align(HorizontalAlign::Left)
                    .line_breaker(BuiltInLineBreaker::AnyCharLineBreaker),
            )
            .with_screen_position((10., 10.))
            .to_owned();

        let this = Self {
            image: Arc::new(image),

            window,

            surface_config,

            wgpu_instance,
            surface,
            adapter,

            device, queue,
            
            render_pipeline,
            brush,
            debug_text,

            viewport_bind_group_layout,
            viewport_bind_group,
            viewport_transform_buffer,

            image_section_bind_group_layout,
            image_sections: vec![],

            camera_x: 0.,
            camera_y: 0.,
            camera_zoom: 1.,

            pressed_keys: HashSet::default(),
            just_pressed_keys: HashSet::default(),
        };

        this.start(event_loop);
    }

    fn create_section(&mut self, pos: SectionPosition) {
        let image = Arc::clone(&self.image);
        let is = self.create_raw_section_lazy(pos, move || { async move {
            let i = image.load(pos.subdivisions, pos.pos.0, pos.pos.1).await;
            i.unwrap()
        }});
        self.image_sections.push(is);
    }

    fn create_raw_section_lazy<R, F>(
        &self,
        position: SectionPosition,
        f: R,
    ) -> ImageSection
        where R: FnOnce() -> F + Send + Sync + 'static,
              F: Future<Output = image::RgbaImage> + Send + Sync + 'static
    {
        let loading_image: Arc<(AtomicBool, Mutex<Option<image::RgbaImage>>)>
            = Default::default();

        tokio::spawn({
            let loading_image = Arc::clone(&loading_image);
            async move {
                let i = f().await;
                *loading_image.1.lock().unwrap() = Some(i);
                loading_image.0.store(
                    true, std::sync::atomic::Ordering::Relaxed
                );
            }
        });

        let t_image = image::RgbaImage::new(IMAGE_SECTION_SIZE, IMAGE_SECTION_SIZE);
        self.create_raw_section(position, t_image, loading_image)
    }

    fn create_raw_section(
        &self,
        position: SectionPosition,
        image: image::RgbaImage,
        loading_image: Arc<(AtomicBool, Mutex<Option<image::RgbaImage>>)>,
    ) -> ImageSection {
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 1.,
            lod_max_clamp: 1.,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let texture = self.device.create_texture_with_data(&self.queue,
            &wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: image.width(),
                    height: image.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }, image.as_bytes());

        let texture_view = texture.create_view(
            &wgpu::TextureViewDescriptor { ..Default::default() }
        );

        let scale = 1. / (position.subdivisions as f32);
        let size = scale * 2.;

        let dx = -1. + scale + (position.pos.0 as f32) * size;
        let dy = -1. + scale + (position.pos.1 as f32) * size;

        let transform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of::<[f32; 12]>(&[
                scale,  0.   ,  dx ,    /* PADDING */ 0.,
                0.   ,  scale, -dy ,    /* PADDING */ 0.,
                0.   ,  0.   ,  1. ,    /* PADDING */ 0.,
            ]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.image_section_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(transform_buffer.as_entire_buffer_binding()),
                }
            ],
        });

        ImageSection {
            loading_image,
            hide: true,
            
            image,
            bind_group,
            texture,
            sampler,
            transform_buffer,

            position
        }
    }

    fn update_viewport_transform(&mut self) {
        let size = self.window.inner_size();
        let x = (size.height as f32) / (size.width as f32);
        self.queue.write_buffer(&self.viewport_transform_buffer, 0,
            bytemuck::bytes_of::<[f32; 12]>(&[
                x * self.camera_zoom  , 0.              , -self.camera_x * x * self.camera_zoom,    /* PADDING */ 0.,
                0.                    , self.camera_zoom, -self.camera_y * self.camera_zoom,    /* PADDING */ 0.,
                0.                    , 0.              , 1.                               ,    /* PADDING */ 0.,
            ])
        );
    }

    fn update(&mut self, control_flow: &mut ControlFlow, elapsed: Duration) {
        let dt = elapsed.as_secs_f32();

        let camera_move_speed = (dt * 4.) / self.camera_zoom;

        if self.pressed_keys.contains(&VirtualKeyCode::Up)
        { self.camera_y += camera_move_speed; }
        if self.pressed_keys.contains(&VirtualKeyCode::Down)
        { self.camera_y -= camera_move_speed; }

        if self.pressed_keys.contains(&VirtualKeyCode::Left)
        { self.camera_x -= camera_move_speed; }
        if self.pressed_keys.contains(&VirtualKeyCode::Right)
        { self.camera_x += camera_move_speed; }

        if self.just_pressed_keys.contains(&VirtualKeyCode::R) {
            self.camera_x = 0.;
            self.camera_y = 0.;
            self.camera_zoom = 1.;
        }
        if self.just_pressed_keys.contains(&VirtualKeyCode::Q) {
            control_flow.set_exit();
        }

        for is in self.image_sections.iter_mut() {
            if is.loading_image.0.load(std::sync::atomic::Ordering::Relaxed) {
                is.loading_image.0
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                is.hide = false;
                let ni = is.loading_image.1.lock().unwrap().take().unwrap();
                self.queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &is.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    ni.as_bytes(),
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(ni.width() * 4),
                        rows_per_image: Some(ni.height() * 4),
                    },
                    wgpu::Extent3d {
                        width: ni.width(),
                        height: ni.height(),
                        depth_or_array_layers: 1,
                    }
                );
            }
        }

        let mut height_pixel_density = IMAGE_SECTION_SIZE as f32 / (self.window.inner_size().height as f32 * self.camera_zoom);
        let mut subdivis = 1u32;
        while height_pixel_density < 0.8 {
            subdivis *= 2;
            height_pixel_density *= 2.;
        }
        subdivis = Ord::min(
            subdivis,
            self.image.max_level_available().unwrap_or(subdivis)
        );

        self.image_sections.extract_if(|x| {
            x.position.subdivisions > subdivis
        }).for_each(|_| ());

        let screen_top =    self.camera_y + 1. / self.camera_zoom;
        let screen_bottom = self.camera_y - 1. / self.camera_zoom;
        let screen_left =   self.camera_x - 1. / self.camera_zoom;
        let screen_right =  self.camera_x + 1. / self.camera_zoom;

        let mut s = 1u32;

        let mut subdivisions: HashSet<(u32, u32)> =
            (0..1).cartesian_product(0..1).collect();
        while s <= subdivis {
            if !self.image.is_level_available(s) { s *= 2; continue };

            let mut next_subdivisions = HashSet::<(u32, u32)>::default();

            for (sx, sy) in subdivisions.iter().copied() {
                let size = 2. / s as f32;
                let s_top    =  1. - size * sy as f32;
                let s_bottom =  s_top - size;
                let s_right  =  1. - size * (s - sx - 1) as f32;
                let s_left   =  s_right - size;

                let touch = s_left < screen_right && s_right > screen_left &&
                            s_top > screen_bottom && s_bottom < screen_top;
                if touch {
                    next_subdivisions.extend(
                        ((sx*2)..=(sx*2+1))
                            .cartesian_product((sy*2)..=(sy*2+1))
                    );

                    let present = self.image_sections.iter()
                        .map(|is| is.position)
                        .any(|p|
                            p.subdivisions == s &&
                            p.pos.0 == sx && p.pos.1 == sy
                        );
                    if !present {
                        println!("Create {s}_{sx}x{sy}");
                        self.create_section(SectionPosition {
                            subdivisions: s, pos: (sx, sy)
                        });
                    }
                }
            }

            subdivisions = next_subdivisions;

            s *= 2;
        }

        self.debug_text = self.debug_text.clone().with_text(vec![
            OwnedText::new(format!("Rendering: {}", self.image.render_queue_length()))
                .with_scale(50.)
                .with_color([0., 0., 0., 2.])
        ]);

        self.just_pressed_keys.clear();
    }

    pub fn start(mut self, event_loop: EventLoop<()>) {
        let render_interval = Duration::from_secs_f32(1. / 60.);
        let update_interval = Duration::from_secs_f32(1. / 30.);

        let mut last_render = Instant::now();
        let mut last_update = Instant::now();

        event_loop.run(move |event, _, control_flow| {
            control_flow.set_wait_until(Ord::min(
                last_render + render_interval,
                last_update + update_interval
            ));

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    println!("The close button was pressed; stopping");
                    control_flow.set_exit();
                },
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    self.surface_config.width = size.width;
                    self.surface_config.height = size.height;
                    self.surface.configure(&self.device, &self.surface_config);
                    
                    self.debug_text.bounds = (
                        self.surface_config.width as f32 * 0.4,
                        self.surface_config.height as _
                    );
                    self.brush.resize_view(
                        self.surface_config.width as f32,
                        self.surface_config.height as f32,
                        &self.queue
                    );

                    self.window.request_redraw();
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput {
                        device_id: _, input, is_synthetic: _
                    },
                    ..
                } => {
                    let Some(kc) = input.virtual_keycode else { return };
                    match input.state {
                        ElementState::Pressed => {
                            self.pressed_keys.insert(kc);
                            self.just_pressed_keys.insert(kc);
                        },
                        ElementState::Released => {
                            self.pressed_keys.remove(&kc);
                        },
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::MouseWheel {
                        delta, ..
                    },
                    ..
                } => {
                    let MouseScrollDelta::LineDelta(_, motion) = delta
                        else { return };
                    self.camera_zoom *= 1.2f32.powf(motion);
                }
                Event::MainEventsCleared => {
                    self.device.poll(wgpu::Maintain::Poll);

                    if last_render.elapsed() > render_interval {
                        last_render = Instant::now();
                        self.window.request_redraw();
                    }
                    let elapsed_update = last_update.elapsed();
                    if elapsed_update > update_interval {
                        last_update = Instant::now();
                        self.update(control_flow, elapsed_update);
                    }
                },
                Event::RedrawRequested(_) => {
                    self.update_viewport_transform();
                    self.redraw();
                },
                _ => ()
            }
        });
    }

    pub fn redraw(&mut self) {
        self.brush.queue(&self.debug_text);
        self.brush.process_queued(&self.device, &self.queue).unwrap();

        let frame = self.surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: None }
        );

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.viewport_bind_group, &[]);
            self.image_sections.sort_unstable_by_key(|p| p.position.subdivisions);
            for is in self.image_sections.iter() {
                if is.hide { continue }
                rpass.set_bind_group(1, &is.bind_group, &[]);
                rpass.draw(0..6, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.submit(Some(
            self.brush.draw(&self.device, &view)
        ));

        frame.present();
    }
}

