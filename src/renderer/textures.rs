use image::DynamicImage;
use wgpu::{
    AddressMode, Extent3d, FilterMode, ImageDataLayout, SamplerDescriptor, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
};

pub(crate) struct TextureFactory<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

impl<'a> TextureFactory<'a> {
    pub(crate) fn new(device: &'a wgpu::Device, queue: &'a wgpu::Queue) -> Self {
        Self { device, queue }
    }

    pub(crate) fn output(&self, width: u32, height: u32) -> wgpu::Texture {
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

    pub(crate) fn input(&self, image: Option<&DynamicImage>) -> (wgpu::Texture, wgpu::TextureView) {
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

    pub(crate) fn sampler(&self) -> wgpu::Sampler {
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
}
