mod cli;
mod info;

use clap::Parser;
use cli::Cli;
use image::DynamicImage;
use std::fs;
use std::io::{self, Cursor, IsTerminal, Write};
use std::path::PathBuf;
use tiedye::{TiedyeError, apply_shader};

fn main() -> Result<(), TiedyeError> {
    let cli = Cli::parse();

    if cli.info {
        info::print_info();
        return Ok(());
    }

    // Safe: clap enforces shader is present when --info is absent.
    let shader_path = cli.shader.unwrap();
    let shader_source = fs::read_to_string(&shader_path)
        .map_err(|_| TiedyeError::ShaderNotFound(shader_path.display().to_string()))?;

    let input_images = cli
        .images
        .iter()
        .map(|path| image::open(path).map_err(TiedyeError::from))
        .collect::<Result<Vec<DynamicImage>, _>>()?;

    let output_image = apply_shader(&shader_source, &input_images)?;

    if let Some(output_path) = cli.output {
        output_image.save(&output_path).map_err(TiedyeError::from)?;
        eprintln!("Output written to {}", output_path.display());
    } else if !io::stdout().is_terminal() {
        let mut buf = Vec::new();
        output_image
            .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(TiedyeError::from)?;
        io::stdout()
            .lock()
            .write_all(&buf)
            .map_err(TiedyeError::from)?;
    } else {
        output_image
            .save(PathBuf::from("output.png"))
            .map_err(TiedyeError::from)?;
        eprintln!("Output written to output.png");
    }

    Ok(())
}
