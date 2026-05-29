use bytemuck::{Pod, Zeroable};
use image::{DynamicImage, RgbaImage};
use wgpu::util::DeviceExt;
use wgpu::{
    AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, BufferBindingType,
    BufferDescriptor, BufferUsages, COPY_BYTES_PER_ROW_ALIGNMENT, Color, ColorTargetState,
    ColorWrites, CommandEncoderDescriptor, DeviceDescriptor, Extent3d, FilterMode, FragmentState,
    ImageCopyBuffer, ImageDataLayout, InstanceDescriptor, LoadOp, Maintain, MapMode,
    MultisampleState, Operations, PipelineLayoutDescriptor, PowerPreference, PrimitiveState,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
    RequestAdapterOptions, SamplerBindingType, SamplerDescriptor, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, StoreOp, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension, VertexState,
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

    pub fn render(
        &self,
        shader_source: &str,
        input_images: &[DynamicImage],
    ) -> Result<RgbaImage, TiedyeError> {
        let (width, height) = input_images
            .first()
            .map(|img| (img.width(), img.height()))
            .unwrap_or((512, 512));

        let output_texture = self.create_output_texture(width, height);
        let output_view = output_texture.create_view(&TextureViewDescriptor::default());

        let (input_texture, input_view) = self.create_input_texture(input_images.first());
        let sampler = self.create_sampler();

        let shader = self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("shader"),
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let uniform_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("params"),
                contents: bytemuck::bytes_of(&Params { width, height }),
                usage: BufferUsages::UNIFORM,
            });

        let bind_group_layout = self.create_bind_group_layout();
        let bind_group =
            self.create_bind_group(&bind_group_layout, &uniform_buffer, &input_view, &sampler);

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self.create_pipeline(&shader, &pipeline_layout);

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

        let (output_buffer, padded_bytes_per_row) =
            self.copy_texture_to_buffer(&mut encoder, &output_texture, width, height);

        self.queue.submit(std::iter::once(encoder.finish()));

        // Keep input_texture alive until submission is complete.
        drop(input_texture);

        self.read_back_image(output_buffer, width, height, padded_bytes_per_row)
    }

    fn create_output_texture(&self, width: u32, height: u32) -> wgpu::Texture {
        self.device.create_texture(&TextureDescriptor {
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
        })
    }

    fn create_input_texture(
        &self,
        image: Option<&DynamicImage>,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let (width, height, pixels): (u32, u32, Vec<u8>) = match image {
            Some(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                (w, h, rgba.into_raw())
            }
            None => (1, 1, vec![0, 0, 0, 255]),
        };

        let texture = self.device.create_texture(&TextureDescriptor {
            label: Some("input_texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            texture.as_image_copy(),
            &pixels,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: None,
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_sampler(&self) -> wgpu::Sampler {
        self.device.create_sampler(&SamplerDescriptor {
            label: Some("input_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        })
    }

    fn create_bind_group_layout(&self) -> wgpu::BindGroupLayout {
        self.device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }

    fn create_bind_group(
        &self,
        layout: &wgpu::BindGroupLayout,
        uniform_buffer: &wgpu::Buffer,
        input_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("bind_group"),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(input_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    fn create_pipeline(
        &self,
        shader: &wgpu::ShaderModule,
        layout: &wgpu::PipelineLayout,
    ) -> wgpu::RenderPipeline {
        self.device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("pipeline"),
                layout: Some(layout),
                vertex: VertexState {
                    module: shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(FragmentState {
                    module: shader,
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
            })
    }

    fn copy_texture_to_buffer(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> (wgpu::Buffer, u32) {
        let padded_bytes_per_row =
            (width * 4).div_ceil(COPY_BYTES_PER_ROW_ALIGNMENT) * COPY_BYTES_PER_ROW_ALIGNMENT;

        let buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("output_buffer"),
            size: (padded_bytes_per_row * height) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            ImageCopyBuffer {
                buffer: &buffer,
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

        (buffer, padded_bytes_per_row)
    }

    fn read_back_image(
        &self,
        buffer: wgpu::Buffer,
        width: u32,
        height: u32,
        padded_bytes_per_row: u32,
    ) -> Result<RgbaImage, TiedyeError> {
        let slice = buffer.slice(..);
        slice.map_async(MapMode::Read, |_| {});
        self.device.poll(Maintain::Wait);

        let mapped = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * padded_bytes_per_row) as usize;
            pixels.extend_from_slice(&mapped[start..start + (width * 4) as usize]);
        }
        drop(mapped);
        buffer.unmap();

        RgbaImage::from_raw(width, height, pixels).ok_or_else(|| {
            TiedyeError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "failed to create image from GPU output",
            ))
        })
    }
}
