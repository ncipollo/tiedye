use image::RgbaImage;
use wgpu::{
    BufferDescriptor, BufferUsages, COPY_BYTES_PER_ROW_ALIGNMENT, Extent3d, ImageCopyBuffer,
    ImageDataLayout, Maintain, MapMode,
};

use crate::error::TiedyeError;

pub(crate) struct OutputReader<'a> {
    device: &'a wgpu::Device,
}

impl<'a> OutputReader<'a> {
    pub(crate) fn new(device: &'a wgpu::Device) -> Self {
        Self { device }
    }

    pub(crate) fn copy_to_buffer(
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

    pub(crate) fn read_image(
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
