use std::{borrow::Cow, path::Path};
use wgpu::{util::DeviceExt, PowerPreference};

#[derive(Debug, Clone, Copy)]
pub struct SectionInfo {
    pub subdivisions: u32,
    pub subdiv_pos: (u32, u32),
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,

    bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,

    size: u32,
}

impl Renderer {
    pub async fn new(
        size: u32,
        shader: impl AsRef<Path>,
    ) -> Self {
        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let adapter = wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await.unwrap();

        log::info!("Using adapter:      {:?}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits {
                        max_texture_dimension_2d: size,
                        ..wgpu::Limits::downlevel_defaults()
                    },
                },
                None,
            )
            .await
            .unwrap();

        // Loads the shader from WGSL
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                &crate::shader_prep::preproces_file(shader.as_ref()).await
                    .expect("Could not read shader file")
            )),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                }
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vertex_main",
                buffers: &[],
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
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            multiview: None,
        });

        Self {
            device,
            queue,

            bind_group_layout,
            render_pipeline,

            size,
        }
    }

    pub async fn render_section(&self, section: SectionInfo) -> image::RgbaImage {
        let uv_scale = 1. / (section.subdivisions as f32);
        let uv_span = 1. - 1. / (section.subdivisions as f32);
        let uv_x = section.subdiv_pos.0 as f32;
        let uv_y = (section.subdivisions - section.subdiv_pos.1 - 1) as f32;

        let uv_transform_buffer = self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(&[
                    uv_scale, 0., uv_scale * uv_x * 2. - uv_span, 0.,
                    0.,   uv_scale, uv_scale * uv_y * 2. - uv_span, 0.,
                    0.,   0., 1., 0.,
                ]),
                usage: wgpu::BufferUsages::UNIFORM
            });
        let screen_size_buffer = self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(&[
                    section.subdivisions * self.size,
                    section.subdivisions * self.size
                ]),
                usage: wgpu::BufferUsages::UNIFORM
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uv_transform_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: screen_size_buffer.as_entire_binding()
                }
            ]
        });

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: self.size, height: self.size, depth_or_array_layers: 1
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm]
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let pixel_size = texture.format().block_size(None).expect("Invalid format");

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size:
                (self.size as u64).pow(2) * pixel_size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: &texture_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                store: true,
                            },
                        })
                    ],
                    depth_stencil_attachment: None,
                }
            );
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTextureBase {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBufferBase {
                buffer: &staging_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row:  Some(pixel_size * self.size),
                    rows_per_image: Some(pixel_size * self.size),
                }
            },
            texture.size()
        );

        let submition_id = self.queue.submit(Some(encoder.finish()));
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

        log::debug!("Waiting for render to finish...");

        tokio::task::block_in_place(move || {
            self.device.poll(
                wgpu::Maintain::WaitForSubmissionIndex(submition_id)
            );
        });

        if let Some(Ok(())) = receiver.receive().await {
            let data = buffer_slice.get_mapped_range();
            let result = data.to_vec();

            drop(data);
            staging_buffer.unmap();

            let image = image::RgbaImage::from_raw(
                self.size, self.size, result
            ).unwrap();

            image
        } else {
            panic!("failed to run compute on gpu!")
        }
    }
  
}
