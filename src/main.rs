use anyhow::{Context, Result};
use rand::Rng;
use rayon::prelude::*;
use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

mod camera;
mod hitting;
mod math;
mod progress;

use camera::Camera;
use hitting::{Hittable, Sphere};
use math::{Point3, Ray, Vec3};
use progress::Progress;

type Colour = Vec3;

fn main() -> Result<()> {
    // Output
    let mut output = io::stdout();
    let mut info = io::stderr();

    // Image
    let aspect_ratio = 16.0 / 9.0;
    let image_width = 400;
    let image_height = (image_width as f64 / aspect_ratio).round() as u32;
    let viewport_height = 2.0;
    let focal_length = 3.0;

    // UI
    let progress_bar_len = 80;

    let camera = Camera::new(image_width, image_height, viewport_height, focal_length);

    let start_time = Instant::now();

    let world: Vec<Box<dyn Hittable>> = vec![
        Sphere {
            centre: Point3::new(0, 0, -3),
            radius: 0.5,
        },
        Sphere {
            centre: Point3::new(-1.5, 0.5, -4.0),
            radius: 0.5,
        },
        Sphere {
            centre: Point3::new(1, 1, -4),
            radius: 0.5,
        },
        Sphere {
            centre: Point3::new(0.0, -100.5, -1.0),
            radius: 100.0,
        },
    ]
    .into_iter()
    .map(|s: Sphere| -> Box<dyn Hittable> { Box::new(s) })
    .collect();

    let samples_per_pixel = 100;
    let max_bounces = 50;

    // Print progress
    let (progress_sender, progress_receiver): (Sender<()>, Receiver<()>) = mpsc::channel();
    let (done_sender, done_receiver): (Sender<Result<()>>, Receiver<Result<()>>) = mpsc::channel();
    thread::spawn(move || {
        let mut progress = Progress::new(&mut info, progress_bar_len);
        progress.set_label("Rendering");
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

    // Render in parallel
    let pixels = (0..image_height)
        .rev()
        .map(|j| (j, progress_sender.clone()))
        .collect::<Vec<(u32, mpsc::Sender<()>)>>()
        //.iter()
        .into_par_iter()
        .map(|(j, sender)| -> Result<Vec<Colour>> {
            let mut rng = rand::thread_rng();
            let mut row = Vec::with_capacity(image_width as usize);
            for i in 0..image_width {
                let mut colour = Vec3::new(0, 0, 0);
                for _ in 0..samples_per_pixel {
                    let u = (i as f64 + rng.gen_range(0.0..1.0)) / (image_width - 1) as f64;
                    let v = (j as f64 + rng.gen_range(0.0..1.0)) / (image_height - 1) as f64;
                    let r = camera.cast_ray(u, v);
                    colour += ray_colour(&r, &world, max_bounces, &mut rng);
                }
                colour /= samples_per_pixel as f64;
                row.push(colour);
            }
            sender.send(()).unwrap();
            return Ok(row);
        })
        .collect::<Result<Vec<Vec<Colour>>>>()?;

    done_receiver.recv()??;
    let mut info = io::stderr();
    let mut progress = Progress::new(&mut info, progress_bar_len);
    progress.set_label("Printing");
    output.write(format!("P3\n{} {}\n255\n", image_width, image_height).as_bytes())?;
    for (i, row) in pixels.iter().enumerate() {
        progress.update(i, pixels.len())?;
        for &colour in row {
            write_pixel(&mut output, colour)?;
        }
    }
    progress.clear()?;

    eprintln!(
        "Completed in {:.2} seconds.",
        start_time.elapsed().as_secs_f32()
    );

    Ok(())
}

fn ray_colour<T: Hittable, U: Rng>(ray: &Ray, world: &T, bounces: u32, rng: &mut U) -> Colour {
    if let Some(hit) = world.hit(ray, 0.0, f64::INFINITY) {
        let target = hit.intersection + hit.normal + random_in_unit_sphere(rng);
        return if bounces > 0 {
            0.5 * ray_colour(
                &Ray {
                    origin: hit.intersection,
                    direction: target,
                },
                world,
                bounces - 1,
                rng,
            )
        } else {
            Colour::new(0, 0, 0)
        };
    }
    let unit_direction = ray.direction.unit_vector();
    let t = 0.5 * (unit_direction.y + 1.0);
    (1.0 - t) * Colour::new(1, 1, 1) + t * Colour::new(0.5, 0.7, 1.0)
}

fn random_in_unit_sphere<T: Rng>(rng: &mut T) -> Vec3 {
    loop {
        let p = Vec3::new(
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
        );
        if p.length_squared() < 1.0 {
            return p;
        }
    }
}

fn write_pixel<T: Write>(output: &mut T, c: Colour) -> Result<()> {
    let r = (255.0 * clamp(c.x.abs(), 0.0, 0.999)).floor() as u32;
    let g = (255.0 * clamp(c.y.abs(), 0.0, 0.999)).floor() as u32;
    let b = (255.0 * clamp(c.z.abs(), 0.0, 0.999)).floor() as u32;
    output
        .write(format!("{} {} {}\n", r, g, b).as_bytes())
        .and(Ok(()))
        .context(format!("Writing pixel {}", c))
}

fn clamp(a: f64, min: f64, max: f64) -> f64 {
    if a < min {
        min
    } else if a > max {
        max
    } else {
        a
    }
}
