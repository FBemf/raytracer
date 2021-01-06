use anyhow::{Context, Result};
use image::{ImageBuffer, RgbImage};
use rand::Rng;
use rayon::prelude::*;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use structopt::StructOpt;

mod camera;
mod hitting;
mod materials;
mod math;
mod objects;
mod progress;

use camera::Camera;
use hitting::{cast_ray, random_colour, Colour, Hittable, Material};
use materials::{Dielectric, Lambertian, Luminescent, LuminescentMetal, Metal};
use math::{clamp, coeff, Point3, Ray, Vec3};
use objects::Sphere;
use progress::{Progress, TimedProgressBar};

#[derive(Debug, StructOpt)]
#[structopt(name = "raytracer", about = "Raytracing in a weekend!")]
struct Opt {
    /// Output file
    #[structopt(parse(from_os_str))]
    file: PathBuf,
}

fn main() -> Result<()> {
    // cli args
    let opt = Opt::from_args();

    // Output streams
    let mut info = io::stderr();

    // Image
    let aspect_ratio = 3.0 / 2.0;
    let image_width = 1200;
    let image_height = (image_width as f64 / aspect_ratio).round() as u32;

    // UI
    let progress_bar_len = 60;
    let start_time = Instant::now();

    // Camera
    let look_from = Point3::new(13, 2, 3);
    let look_at = Point3::new(0, 0, 0);
    let direction_up = Vec3::new(0, 1, 0);
    let field_of_view = 20;
    let aperture = 0.1;
    let distance_to_focus = 10.0;
    let camera = Camera::new(
        look_from,
        look_at,
        direction_up,
        field_of_view,
        aspect_ratio,
        aperture,
        distance_to_focus,
    );

    let world = random_scene();

    let samples_per_pixel = 100;
    let max_bounces = 50;

    // Print progress
    let (progress_sender, progress_receiver): (Sender<()>, Receiver<()>) = mpsc::channel();
    let (done_sender, done_receiver): (Sender<Result<()>>, Receiver<Result<()>>) = mpsc::channel();
    thread::spawn(move || {
        let mut progress = TimedProgressBar::new(
            &mut info,
            progress_bar_len,
            "Rendering",
            " -=≡",
            start_time.clone(),
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
                    colour += cast_ray(&r, &world, background, max_bounces);
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

    done_receiver.recv()??;

    let img: RgbImage = ImageBuffer::from_raw(image_width, image_height, pixels).unwrap();
    img.save(opt.file)?;

    let elapsed = start_time.elapsed().as_secs();
    eprintln!("Completed in {}:{:02}", elapsed / 60, elapsed % 60,);

    Ok(())
}

fn background(ray: &Ray) -> Colour {
    let unit_direction = ray.direction.unit_vector();
    let t = 0.5 * (unit_direction.y + 1.0);
    0.1 * ((1.0 - t) * Colour::new(1, 1, 1) + t * Colour::new(0.5, 0.7, 1.0))
}

fn colour_to_raw(c: Colour) -> Vec<u8> {
    let r = (255.0 * clamp(c.x.abs(), 0.0, 0.999)).floor() as u8;
    let g = (255.0 * clamp(c.y.abs(), 0.0, 0.999)).floor() as u8;
    let b = (255.0 * clamp(c.z.abs(), 0.0, 0.999)).floor() as u8;
    vec![r, g, b]
}

fn random_scene() -> Vec<Box<dyn Hittable>> {
    // Materials
    let material_bright: Arc<dyn Material> = Arc::new(Luminescent {
        light_colour: Colour::new(1.0, 1.0, 0.7),
    });
    let material_ground: Arc<dyn Material> = Arc::new(Lambertian {
        albedo: Colour::new(0.5, 0.5, 0.5),
    });
    let material_glass: Arc<dyn Material> = Arc::new(Dielectric {
        index_of_refraction: 1.5,
    });
    let material_matte: Arc<dyn Material> = Arc::new(Lambertian {
        albedo: Colour::new(0.4, 0.2, 0.1),
    });
    let material_metal: Arc<dyn Material> = Arc::new(Metal {
        albedo: Colour::new(0.7, 0.6, 0.5),
        fuzz: 0.0,
    });

    // World
    let mut world = Vec::new();

    world.push(Sphere::new(Point3::new(2, 12, 4), 5.0, &material_bright));
    world.push(Sphere::new(
        Point3::new(0, -1000, 0),
        1000.0,
        &material_ground,
    ));
    world.push(Sphere::new(Point3::new(0, 1, 0), 1.0, &material_glass));
    world.push(Sphere::new(Point3::new(-4, 1, 0), 1.0, &material_matte));
    world.push(Sphere::new(Point3::new(4, 1, 0), 1.0, &material_metal));

    let mut rng = rand::thread_rng();
    let sparsity = 1.0;

    for a in -11..11 {
        for b in -11..11 {
            if rng.gen_range(0.0..1.0) <= sparsity {
                let choose_mat = rng.gen_range(0.0..1.0);
                let centre = Point3::new(
                    a as f64 + 0.9 * rng.gen_range(0.0..1.0),
                    0.2,
                    b as f64 + 0.9 * rng.gen_range(0.0..1.0),
                );
                if (centre - Point3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                    let material: Arc<dyn Material> = if choose_mat < 0.4 {
                        let albedo = coeff(random_colour(0, 1), random_colour(0, 1));
                        Arc::new(Lambertian { albedo })
                    } else if choose_mat < 0.7 {
                        let albedo = random_colour(0.5, 1.0);
                        let fuzz = rng.gen_range(0.0..0.5);
                        Arc::new(Metal { albedo, fuzz })
                    } else if choose_mat < 0.85 {
                        Arc::clone(&material_glass)
                    } else if choose_mat < 0.95 {
                        let colour = random_colour(0.0, 1.0);
                        Arc::new(LuminescentMetal::with_colour(colour, 0.0, 0.6))
                    } else {
                        Arc::clone(&material_bright)
                    };
                    world.push(Sphere::new(centre, 0.2, &material));
                }
            }
        }
    }

    world
}
