#![feature(int_roundings)]

use std::{borrow::Cow, str::FromStr, time::Duration};
use wgpu::{util::DeviceExt, PowerPreference};

async fn run() {
    let size = 2u32.pow(13);
    log::info!("Using size {size:?}");
    let output = execute_gpu(size).await.unwrap();

    log::info!("Saving image...");
    image::save_buffer(
        "image.bmp", output.as_slice(), size, size, image::ColorType::Rgba8
    ).unwrap();
    log::info!("Finished !");
}

async fn execute_gpu(size: u32) -> Option<Vec<u8>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits {
                    max_texture_dimension_2d: size,
                    max_buffer_size: size as u64 * size as u64 * 4,
                    ..wgpu::Limits::downlevel_defaults()
                },
            },
            None,
        )
        .await
        .unwrap();

    let info = adapter.get_info();
    log::info!("Using Adapter: {:?}", info.name);
    if info.vendor == 0x10005 {
        return None;
    }

    execute_gpu_inner(&device, &queue, size).await
}

async fn execute_gpu_inner(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: u32,
) -> Option<Vec<u8>> {
    // Loads the shader from WGSL
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: size, height: size, depth_or_array_layers: 1
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

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size:
            (size as u64).pow(2) * pixel_size as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
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
            }
        ],
    });
    let uv_transform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(&[
            0.5f32, 0., 0., 0.,
            0.,   0.5, 0., 0.,
            0.,   0., 1., 0.,
        ]),
        usage: wgpu::BufferUsages::UNIFORM
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uv_transform_buffer.as_entire_binding()
            }
        ]
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

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
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
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.set_pipeline(&render_pipeline);
        render_pass.draw(0..6, 0..1)
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
                bytes_per_row: Some(pixel_size * size),
                rows_per_image: Some(pixel_size * size),
            }
        },
        texture.size()
    );

    // Submits command encoder for processing
    queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    device.poll(wgpu::Maintain::Wait);

    // Awaits until `buffer_future` can be read from
    if let Some(Ok(())) = receiver.receive().await {
        log::info!("Render finish, reading image...");
        let data = buffer_slice.get_mapped_range();
        let result = data.to_vec();

        drop(data);
        staging_buffer.unmap();
        Some(result)
    } else {
        panic!("failed to run compute on gpu!")
    }
}

fn main() {
    env_logger::builder()
        .filter(None, log::LevelFilter::Warn)
        .filter_module("fractals", log::LevelFilter::Trace)
        .init();
    pollster::block_on(run());
}
