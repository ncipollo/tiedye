mod output;
mod pipeline;
mod textures;

use image::{DynamicImage, RgbaImage};
use output::OutputReader;
use pipeline::PipelineBuilder;
use textures::TextureFactory;
use wgpu::{
    Color, CommandEncoderDescriptor, DeviceDescriptor, InstanceDescriptor, LoadOp, Operations,
    PowerPreference, RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions,
    StoreOp, TextureViewDescriptor,
};

use crate::error::TiedyeError;

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
        let textures = TextureFactory::new(&self.device, &self.queue);
        let builder = PipelineBuilder::new(&self.device);
        let reader = OutputReader::new(&self.device);

        // Size output to the first input image, defaulting to 512x512.
        let (width, height) = input_images
            .first()
            .map(|img| (img.width(), img.height()))
            .unwrap_or((512, 512));

        // GPU resources: render target, input texture/sampler, shader, uniforms.
        let output_texture = textures.output(width, height);
        let output_view = output_texture.create_view(&TextureViewDescriptor::default());
        let (input_texture, input_view) = textures.input(input_images.first());
        let sampler = textures.sampler();
        let shader = builder.shader(shader_source);
        let uniform_buffer = builder.uniform_buffer(width, height);

        // Bindings and pipeline.
        let bind_group_layout = builder.bind_group_layout();
        let bind_group =
            builder.bind_group(&bind_group_layout, &uniform_buffer, &input_view, &sampler);
        let pipeline_layout = builder.pipeline_layout(&bind_group_layout);
        let pipeline = builder.pipeline(&shader, &pipeline_layout);

        // Encode a single full-screen triangle draw into the output texture.
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

        // Copy rendered output to a mappable buffer and read pixels back to CPU.
        let (output_buffer, padded_bytes_per_row) =
            reader.copy_to_buffer(&mut encoder, &output_texture, width, height);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Keep input_texture alive until submission is complete.
        drop(input_texture);

        reader.read_image(output_buffer, width, height, padded_bytes_per_row)
    }
}
