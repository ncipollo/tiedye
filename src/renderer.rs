use bytemuck::{Pod, Zeroable};
use image::{DynamicImage, RgbaImage};
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BlendState, BufferBindingType, BufferDescriptor, BufferUsages,
    COPY_BYTES_PER_ROW_ALIGNMENT, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor,
    DeviceDescriptor, Extent3d, FragmentState, ImageCopyBuffer, ImageDataLayout,
    InstanceDescriptor, LoadOp, Maintain, MapMode, MultisampleState, Operations,
    PipelineLayoutDescriptor, PowerPreference, PrimitiveState, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, StoreOp, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor, VertexState,
};

use crate::error::TiedyeError;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Params {
    width: u32,
    height: u32,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Renderer {
    pub fn new() -> Result<Self, TiedyeError> {
        pollster::block_on(Self::new_async())
    }

    async fn new_async() -> Result<Self, TiedyeError> {
        let instance = wgpu::Instance::new(InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or(TiedyeError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default(), None)
            .await?;

        Ok(Self { device, queue })
    }

    #[allow(clippy::too_many_lines)]
    pub fn render(
        &self,
        shader_source: &str,
        input_images: &[DynamicImage],
    ) -> Result<RgbaImage, TiedyeError> {
        let (width, height) = input_images
            .first()
            .map(|img| (img.width(), img.height()))
            .unwrap_or((512, 512));

        let params = Params { width, height };

        let output_texture = self.device.create_texture(&TextureDescriptor {
            label: Some("output"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let shader = self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("shader"),
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let uniform_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("params"),
                contents: bytemuck::bytes_of(&params),
                usage: BufferUsages::UNIFORM,
            });

        let bind_group_layout = self
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bind_group_layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: TextureFormat::Rgba8Unorm,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("bind_group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let output_view = output_texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(COPY_BYTES_PER_ROW_ALIGNMENT)
            * COPY_BYTES_PER_ROW_ALIGNMENT;

        let output_buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("output_buffer"),
            size: (padded_bytes_per_row * height) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            output_texture.as_image_copy(),
            ImageCopyBuffer {
                buffer: &output_buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = output_buffer.slice(..);
        buffer_slice.map_async(MapMode::Read, |_| {});
        self.device.poll(Maintain::Wait);

        let padded_data = buffer_slice.get_mapped_range();
        let mut image_data = Vec::with_capacity((width * height * 4) as usize);

        for row in 0..height {
            let row_start = (row * padded_bytes_per_row) as usize;
            let row_end = row_start + (width * bytes_per_pixel) as usize;
            image_data.extend_from_slice(&padded_data[row_start..row_end]);
        }

        drop(padded_data);
        output_buffer.unmap();

        RgbaImage::from_raw(width, height, image_data).ok_or_else(|| {
            TiedyeError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "failed to create image from GPU output",
            ))
        })
    }
}
