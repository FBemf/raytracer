use anyhow::{Context, Result};
use image::{ImageBuffer, RgbImage};
use rand::Rng;
use rayon::prelude::*;
use structopt::StructOpt;
use terminal_size::{terminal_size, Height, Width};

use std::fs::remove_file;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

mod camera;
mod config;
mod hitting;
mod materials;
mod math;
mod objects;
mod part_file;
mod progress;
mod textures;
mod transforms;

use config::load_config;
use hitting::{cast_ray, Colour};
use math::{clamp, Vec3};
use part_file::PartFile;
use progress::{Progress, TimedProgressBar};

#[derive(Debug, StructOpt)]
#[structopt(name = "raytracer", about = "Raytracing in a weekend!")]
struct Opt {
    /// Config file
    #[structopt(parse(from_os_str))]
    input_file: PathBuf,
    /// Output file
    #[structopt(parse(from_os_str))]
    output_file: PathBuf,
    /// Output image width
    #[structopt(short, long, default_value = "600")]
    width: u32,
    /// Rays per pixel
    #[structopt(short = "s", long, default_value = "100")]
    ray_samples: u32,
    /// Maximum number of bounces for any ray
    #[structopt(short, long, default_value = "50")]
    max_bounces: u32,
    /// No informational messages printed to stderr
    #[structopt(short, long)]
    quiet: bool,
    /// Do not use non-ASCII symbols
    #[structopt(short, long)]
    ascii_symbols_only: bool,
    /// Recover from part file
    #[structopt(short, long)]
    recover_from: Option<PathBuf>,
    /// Try to read as much of a corrupted part file as possible
    #[structopt(short, long)]
    recover_corrupt: bool,
    /// Manually set length of progress bar
    #[structopt(short, long)]
    progress_bar_len: Option<usize>,
    /// Don't save partial progress in a part file in case of a crash
    #[structopt(short, long)]
    no_part_file: bool,
}

fn main() -> Result<()> {
    // cli args
    let opt = Opt::from_args();

    // Camera & World
    let (camera, world, sky, aspect_ratio) = load_config(&opt.input_file)?;

    // Image
    let image_width = opt.width;
    let image_height = (image_width as f64 / aspect_ratio).round() as u32;

    let render_start = Instant::now();

    let samples_per_pixel = opt.ray_samples;
    let max_bounces = opt.max_bounces;

    // Channels to communicate progress
    let (progress_sender, progress_receiver): (Sender<(u32, Vec<u8>)>, Receiver<(u32, Vec<u8>)>) =
        mpsc::channel();
    let (done_sender, done_receiver): (Sender<Result<()>>, Receiver<Result<()>>) = mpsc::channel();

    // Start a separate thread to run the progress bar and manage the part file
    let progress_info = ProgressInfo {
        output_file_name: opt.output_file.clone(),
        image_width,
        image_height,
        ascii_symbols_only: opt.ascii_symbols_only,
        render_start,
        progress_bar_len: opt.progress_bar_len,
        progress_receiver,
        quiet: opt.quiet,
        no_part_file: opt.no_part_file,
    };
    thread::spawn(move || {
        done_sender.send(monitor_progress(progress_info)).unwrap();
    });

    let base = if let Some(part_file) = opt.recover_from {
        PartFile::read(&part_file, image_height, image_width, opt.recover_corrupt)?
    } else {
        vec![None; image_height as usize]
    };

    // Render in parallel
    let pixels = base
        .into_iter()
        .enumerate()
        .rev()
        .map(|(j, v)| (j as u32, v, progress_sender.clone()))
        .collect::<Vec<(u32, Option<Vec<u8>>, mpsc::Sender<(u32, Vec<u8>)>)>>()
        .into_par_iter()
        .map(|(j, from_part_file, sender)| {
            match from_part_file {
                None => {
                    let mut rng = rand::thread_rng();
                    let mut row = Vec::with_capacity(3 * image_width as usize);
                    for i in 0..image_width {
                        let mut colour = Vec3::new(0, 0, 0);
                        for _ in 0..samples_per_pixel {
                            let u = (i as f64 + rng.gen_range(0.0..1.0)) / (image_width - 1) as f64;
                            let v =
                                (j as f64 + rng.gen_range(0.0..1.0)) / (image_height - 1) as f64;
                            let r = camera.find_ray(u, v);
                            colour += cast_ray(&r, &world, &sky, max_bounces);
                        }
                        colour /= samples_per_pixel as f64;
                        // correct for gamma=2.0 (raise to the power of 1/gamma, i.e. sqrt)
                        let gamma_corrected =
                            Colour::new(colour.x.sqrt(), colour.y.sqrt(), colour.z.sqrt());
                        row.append(&mut colour_to_raw(gamma_corrected));
                    }
                    sender.send((j, row.clone())).unwrap();
                    return row;
                }
                Some(row) => {
                    sender.send((j, row.clone())).unwrap();
                    return row;
                }
            }
        })
        .flatten()
        .collect::<Vec<u8>>();

    if !opt.quiet {
        // join with the progress bar
        done_receiver.recv()??;
    }

    let img: RgbImage = ImageBuffer::from_raw(image_width, image_height, pixels).unwrap();
    img.save(opt.output_file)?;

    if !opt.quiet {
        let elapsed = render_start.elapsed().as_secs();
        eprintln!("Completed in {}:{:02}", elapsed / 60, elapsed % 60,);
    }

    Ok(())
}

fn colour_to_raw(c: Colour) -> Vec<u8> {
    let r = (255.0 * clamp(c.x.abs(), 0.0, 0.999)).floor() as u8;
    let g = (255.0 * clamp(c.y.abs(), 0.0, 0.999)).floor() as u8;
    let b = (255.0 * clamp(c.z.abs(), 0.0, 0.999)).floor() as u8;
    vec![r, g, b]
}

struct ProgressInfo {
    output_file_name: PathBuf,
    image_width: u32,
    image_height: u32,
    ascii_symbols_only: bool,
    render_start: Instant,
    progress_bar_len: Option<usize>,
    progress_receiver: mpsc::Receiver<(u32, Vec<u8>)>,
    quiet: bool,
    no_part_file: bool,
}

fn monitor_progress(info: ProgressInfo) -> Result<()> {
    let mut part_file = if !info.no_part_file {
        Some(PartFile::new(&info.output_file_name)?)
    } else {
        None
    };
    if let Some(file) = &mut part_file {
        file.file
            .write_all(format!("{} {}\n", info.image_width, info.image_height).as_bytes())?;
    }
    let bar_symbols = if info.ascii_symbols_only {
        " .:"
    } else {
        " -=≡"
    };
    let progress_bar_len = if let Some(n) = info.progress_bar_len {
        n
    } else {
        if let Some((Width(w), Height(_))) = terminal_size() {
            usize::min(w as usize, 100)
        } else {
            60
        }
    };
    let mut progress = TimedProgressBar::new(
        io::stderr(),
        progress_bar_len,
        "Rendering",
        bar_symbols,
        info.render_start,
    );
    for j in 0..info.image_height {
        let received = info.progress_receiver.recv().context("Rendering progress");
        if let Ok((line_number, part)) = received {
            if !info.quiet {
                progress.update(j as usize, info.image_height as usize)?;
            }
            if let Some(file) = &mut part_file {
                if let Err(e) = file.write_part(line_number, part) {
                    eprintln!(
                        "\rError writing to part file: {}\nPart file may be corrupted",
                        e
                    );
                }
            }
        }
    }
    if let Some(file) = part_file {
        remove_file(&file.path)?;
    }
    if !info.quiet {
        let _ = progress.clear();
    }
    Ok(())
}
