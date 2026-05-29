use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "tiedye", about = "Apply GPU shaders to create tiedye effects")]
pub struct Cli {
    /// Print usage documentation in markdown (useful for AI agents)
    #[arg(long)]
    pub info: bool,

    /// Path to the WGSL shader file
    #[arg(required_unless_present = "info")]
    pub shader: Option<PathBuf>,

    /// Optional input image(s) to apply the shader to
    pub images: Vec<PathBuf>,

    /// Output file path (defaults to output.png in current directory)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}
