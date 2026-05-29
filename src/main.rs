mod cli;

use clap::Parser;
use cli::Cli;
use image::DynamicImage;
use std::fs;
use std::io::{self, Cursor, IsTerminal, Write};
use std::path::PathBuf;
use tiedye::{TiedyeError, apply_shader};

fn main() -> Result<(), TiedyeError> {
    let cli = Cli::parse();

    let shader_source = fs::read_to_string(&cli.shader)
        .map_err(|_| TiedyeError::ShaderNotFound(cli.shader.display().to_string()))?;

    let input_images = cli
        .images
        .iter()
        .map(|path| image::open(path).map_err(TiedyeError::from))
        .collect::<Result<Vec<DynamicImage>, _>>()?;

    let output_image = apply_shader(&shader_source, &input_images)?;

    let stdout = io::stdout();
    if !stdout.is_terminal() {
        let mut buf = Vec::new();
        output_image
            .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(TiedyeError::from)?;
        let mut writer = io::BufWriter::new(stdout.lock());
        writer.write_all(&buf).map_err(TiedyeError::from)?;
    } else {
        let output_path = cli.output.unwrap_or_else(|| PathBuf::from("output.png"));
        output_image.save(&output_path).map_err(TiedyeError::from)?;
        eprintln!("Output written to {}", output_path.display());
    }

    Ok(())
}
