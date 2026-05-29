use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "tiedye", about = "Apply GPU shaders to create tiedye effects")]
pub struct Cli {
    /// Path to the WGSL shader file
    pub shader: PathBuf,

    /// Optional input image(s) to apply the shader to
    pub images: Vec<PathBuf>,

    /// Output file path (defaults to output.png in current directory)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}
