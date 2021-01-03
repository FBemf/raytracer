use anyhow::{Context, Result};
use std::io::{self, BufWriter, Write};

mod vec3;
use vec3::Vec3;
type Colour = Vec3;

const IMAGE_HEIGHT: usize = 256;
const IMAGE_WIDTH: usize = 256;
const PROGRESS_BAR_LENGTH: usize = 50;

fn create_progress_printer<'a, T: Write>(
    out_stream: &'a mut T,
    bar_len: usize,
) -> Box<dyn FnMut(usize, usize) -> Result<()> + 'a> {
    Box::new(move |p, t| {
        let progress = p * bar_len / t;
        let remainder = bar_len - progress;
        out_stream
            .write(
                format!(
                    "\rProgress: [{}{}] ",
                    "#".repeat(progress),
                    " ".repeat(remainder)
                )
                .as_bytes(),
            )
            .and(Ok(()))
            .context("Writing to progress bar")
    })
}

fn clear_progress_bar<T: Write>(out_stream: &mut T, bar_len: usize) -> Result<()> {
    out_stream
        .write(format!("\r{}\r", " ".repeat(bar_len + 12)).as_bytes())
        .and(Ok(()))
        .context("Clearing progress bar")
}

fn main() -> Result<()> {
    let mut out_stream = BufWriter::new(io::stdout());
    let mut log_stream = io::stderr();
    let result = test_render(
        &mut out_stream,
        IMAGE_WIDTH,
        IMAGE_HEIGHT,
        create_progress_printer(&mut log_stream, PROGRESS_BAR_LENGTH),
    );
    clear_progress_bar(&mut log_stream, PROGRESS_BAR_LENGTH)?;
    eprintln!("Complete");
    return result;
}

fn write_pixel<T: Write>(output: &mut T, c: Colour) -> Result<()> {
    let r = (255.999 * c.x) as u64;
    let g = (255.999 * c.y) as u64;
    let b = (255.999 * c.z) as u64;
    output
        .write(format!("{} {} {}\n", r, g, b).as_bytes())
        .and(Ok(()))
        .context(format!("Writing pixel {}", c))
}

fn test_render<T: Write, U: FnMut(usize, usize) -> Result<()>>(
    output: &mut T,
    width: usize,
    height: usize,
    mut progress: U,
) -> Result<()> {
    output.write(format!("P3\n{} {}\n255\n", width, height).as_bytes())?;
    for j in (0..height).rev() {
        progress(height - j, height)?;
        for i in 0..width {
            let colour = Colour::new(
                i as f64 / (width - 1) as f64,
                j as f64 / (height - 1) as f64,
                0.25,
            );
            write_pixel(output, colour)?;
        }
    }
    Ok(())
}
