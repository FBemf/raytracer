use anyhow::{Context, Result};
use image::{ImageBuffer, RgbImage};
use rand::Rng;
use rayon::prelude::*;
use structopt::StructOpt;

use std::io;
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
mod progress;
mod textures;
mod transforms;

use config::load_config;
use hitting::{cast_ray, Colour};
use math::{clamp, Vec3};
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
}

fn main() -> Result<()> {
    // cli args
    let opt = Opt::from_args();

    // Output streams
    let mut info = io::stderr();

    // Camera & World
    let (camera, world, sky, aspect_ratio) = load_config(&opt.input_file)?;

    // Image
    let image_width = opt.width;
    let image_height = (image_width as f64 / aspect_ratio).round() as u32;

    // UI
    let progress_bar_len = 60;
    let render_start = Instant::now();

    let samples_per_pixel = opt.ray_samples;
    let max_bounces = opt.max_bounces;

    // Channels to communicate progress
    let (progress_sender, progress_receiver): (Sender<()>, Receiver<()>) = mpsc::channel();
    let (done_sender, done_receiver): (Sender<Result<()>>, Receiver<Result<()>>) = mpsc::channel();

    if !opt.quiet {
        // Start a separate thread to run the progress bar
        let bar_symbols = if opt.ascii_symbols_only {
            " .:"
        } else {
            " -=≡"
        };
        thread::spawn(move || {
            let mut progress = TimedProgressBar::new(
                &mut info,
                progress_bar_len,
                "Rendering",
                bar_symbols,
                render_start.clone(),
            );
            for j in 0..image_height {
                let error = progress
                    .update(j as usize, image_height as usize)
                    .and_then(|()| progress_receiver.recv().context("Rendering progress"));
                if let Err(_) = error {
                    done_sender.send(error).unwrap();
                    return;
                }
            }
            let _ = progress.clear();
            done_sender.send(Ok(())).unwrap();
        });
    }

    // Render in parallel
    let pixels = (0..image_height)
        .rev()
        .map(|j| (j, progress_sender.clone()))
        .collect::<Vec<(u32, mpsc::Sender<()>)>>()
        .into_par_iter()
        .map(|(j, sender)| {
            let mut rng = rand::thread_rng();
            let mut row = Vec::with_capacity(3 * image_width as usize);
            for i in 0..image_width {
                let mut colour = Vec3::new(0, 0, 0);
                for _ in 0..samples_per_pixel {
                    let u = (i as f64 + rng.gen_range(0.0..1.0)) / (image_width - 1) as f64;
                    let v = (j as f64 + rng.gen_range(0.0..1.0)) / (image_height - 1) as f64;
                    let r = camera.find_ray(u, v);
                    colour += cast_ray(&r, &world, &sky, max_bounces);
                }
                colour /= samples_per_pixel as f64;
                // correct for gamma=2.0 (raise to the power of 1/gamma, i.e. sqrt)
                let gamma_corrected =
                    Colour::new(colour.x.sqrt(), colour.y.sqrt(), colour.z.sqrt());
                row.append(&mut colour_to_raw(gamma_corrected));
            }
            sender.send(()).unwrap();
            return row;
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
