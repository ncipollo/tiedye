use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BlendState, BufferBindingType, BufferUsages, ColorTargetState,
    ColorWrites, FragmentState, MultisampleState, PipelineLayoutDescriptor, PrimitiveState,
    RenderPipelineDescriptor, SamplerBindingType, ShaderModuleDescriptor, ShaderSource,
    ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Params {
    width: u32,
    height: u32,
}

pub(crate) struct PipelineBuilder<'a> {
    device: &'a wgpu::Device,
}

impl<'a> PipelineBuilder<'a> {
    pub(crate) fn new(device: &'a wgpu::Device) -> Self {
        Self { device }
    }

    pub(crate) fn shader(&self, source: &str) -> wgpu::ShaderModule {
        self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("shader"),
            source: ShaderSource::Wgsl(source.into()),
        })
    }

    pub(crate) fn uniform_buffer(&self, width: u32, height: u32) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("params"),
                contents: bytemuck::bytes_of(&Params { width, height }),
                usage: BufferUsages::UNIFORM,
            })
    }

    pub(crate) fn bind_group_layout(&self) -> wgpu::BindGroupLayout {
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

    pub(crate) fn bind_group(
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

    pub(crate) fn pipeline_layout(
        &self,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::PipelineLayout {
        self.device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("pipeline_layout"),
                bind_group_layouts: &[bind_group_layout],
                push_constant_ranges: &[],
            })
    }

    pub(crate) fn pipeline(
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
}
