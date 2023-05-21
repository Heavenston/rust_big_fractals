
use std::{
    borrow::Cow,
    time::{Duration, Instant},
    collections::HashSet,
    sync::{Arc, Mutex, atomic::AtomicBool}
};

use image::EncodableLayout;
use wgpu::util::DeviceExt;
use winit::{
    event::{Event, WindowEvent, ElementState, VirtualKeyCode, MouseScrollDelta},
    event_loop::{EventLoop, ControlFlow},
    window::{WindowBuilder, Window},
};

const IMAGE_SECTION_SIZE: u32 = 2048;

pub async fn start_app() {
    BigImageApp::new().await
}

#[derive(Debug, Clone, Copy)]
struct SectionPosition {
    pub subdivisions: u32,
    pub pos: (u32, u32),
}

#[derive(Debug)]
struct ImageSection {
    pub loading_image: Arc<(AtomicBool, Mutex<Option<image::RgbaImage>>)>,

    pub image: image::RgbaImage,
    pub bind_group: wgpu::BindGroup,
    pub texture: wgpu::Texture,
    pub sampler: wgpu::Sampler,
    pub transform_buffer: wgpu::Buffer,

    pub position: SectionPosition,
}

#[allow(dead_code)]
struct BigImageApp {
    window: Window,

    surface_config: wgpu::SurfaceConfiguration,

    wgpu_instance: wgpu::Instance,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,

    device: wgpu::Device,
    queue: wgpu::Queue,

    render_pipeline: wgpu::RenderPipeline,

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
    pub async fn new() {
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
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        blend: None,
                        write_mask: wgpu::ColorWrites::all(),
                    })],
                }),
                multiview: None,
            });

        let mut this = Self {
            window,

            surface_config,

            wgpu_instance,
            surface,
            adapter,

            device, queue,
            
            render_pipeline,
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

        let subs = 4;
        for cx in 0..subs {
            for cy in 0..subs {
                this.image_sections.push(this.latent_image_load(
                    SectionPosition {
                        subdivisions: subs,
                        pos: (cx, cy),
                    },
                    || {
                        let i = image::open("image.png").unwrap();
                        let ni = image::imageops::resize(
                            &i, IMAGE_SECTION_SIZE, IMAGE_SECTION_SIZE,
                            image::imageops::Lanczos3
                        );
                        ni
                    }
                ));
            }
        }

        this.start(event_loop);
    }

    fn latent_image_load(
        &self,
        position: SectionPosition,
        f: impl FnOnce() -> image::RgbaImage + Send + Sync + 'static
    ) -> ImageSection {
        let loading_image: Arc<(AtomicBool, Mutex<Option<image::RgbaImage>>)>
            = Default::default();

        rayon::spawn({
            let loading_image = Arc::clone(&loading_image);
            move || {
                let i = f();
                *loading_image.1.lock().unwrap() = Some(i);
                loading_image.0.store(
                    true, std::sync::atomic::Ordering::Relaxed
                );
            }
        });

        let t_image = image::RgbaImage::new(IMAGE_SECTION_SIZE, IMAGE_SECTION_SIZE);
        self.create_image_section(position, t_image, loading_image)
    }

    fn create_image_section(
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
                scale, 0. , dx,    /* PADDING */ 0.,
                0. , scale, -dy,    /* PADDING */ 0.,
                0. , 0. , 1.,    /* PADDING */ 0.,
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
                x * self.camera_zoom  , 0.              , -self.camera_x * self.camera_zoom,    /* PADDING */ 0.,
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
                            self.just_pressed_keys.remove(&kc);
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
            for is in self.image_sections.iter() {
                rpass.set_bind_group(1, &is.bind_group, &[]);
                rpass.draw(0..6, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

