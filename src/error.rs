use thiserror::Error;

#[derive(Debug, Error)]
pub enum TiedyeError {
    #[error("no suitable GPU adapter found")]
    NoAdapter,

    #[error("GPU device error: {0}")]
    DeviceError(#[from] wgpu::RequestDeviceError),

    #[error("image error: {0}")]
    ImageError(#[from] image::ImageError),

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("shader file not found: {0}")]
    ShaderNotFound(String),
}
