pub mod error;
mod renderer;

use image::DynamicImage;
use renderer::Renderer;

pub use error::TiedyeError;

/// Apply a WGSL shader to an optional set of input images and return the rendered image.
///
/// If no input images are provided, renders at 512x512. Otherwise uses the dimensions
/// of the first input image.
pub fn apply_shader(
    shader_source: &str,
    input_images: &[DynamicImage],
) -> Result<DynamicImage, TiedyeError> {
    let renderer = Renderer::new()?;
    let rgba = renderer.render(shader_source, input_images)?;
    Ok(DynamicImage::ImageRgba8(rgba))
}
