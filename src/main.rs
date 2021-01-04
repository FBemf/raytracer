use anyhow::{Context, Result};
use rand::Rng;
use std::io::{self, Write};

mod hitting;
mod math;
mod progress;

use hitting::{Hittable, Sphere};
use math::{Point3, Ray, Vec3};
use progress::Progress;

type Colour = Vec3;

fn main() -> Result<()> {
    // Output
    let mut output = io::stdout();
    let mut info = io::stderr();
    let mut rng = rand::thread_rng();

    // Image
    let aspect_ratio = 16.0 / 9.0;
    let image_width = 400;
    let image_height = (image_width as f64 / aspect_ratio).round() as i32;

    // Camera
    let viewport_height = 2.0;
    let viewport_width = aspect_ratio * viewport_height;
    let focal_length = 1.0;

    let origin = Point3::new(0, 0, 0);
    let horizontal = Vec3::new(viewport_width, 0.0, 0.0);
    let vertical = Vec3::new(0.0, viewport_height, 0.0);
    let lower_left_corner =
        origin - horizontal / 2.0 - vertical / 2.0 - Vec3::new(0.0, 0.0, focal_length);

    let world: Vec<Box<dyn Hittable>> = vec![
        Sphere {
            centre: Point3::new(0, 0, -1),
            radius: 0.5,
        },
        Sphere {
            centre: Point3::new(1, 1, -2),
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

    // Render
    let mut progress = Progress::new(&mut info, 50);
    progress.set_label("Rendering");

    output.write(format!("P3\n{} {}\n255\n", image_width, image_height).as_bytes())?;
    for j in (0..image_height).rev() {
        progress.update(image_height as usize - j as usize, image_height as usize)?;
        for i in 0..image_width {
            let u = i as f64 / (image_width - 1) as f64;
            let v = j as f64 / (image_height - 1) as f64;
            let r = Ray {
                origin,
                direction: lower_left_corner + u * horizontal + v * vertical - origin,
            };
            let colour = ray_colour(&r, &world);
            write_pixel(&mut output, colour)?;
        }
    }

    progress.clear()?;
    eprintln!("Done");
    Ok(())
}

fn ray_colour<T: Hittable>(ray: &Ray, world: &T) -> Colour {
    if let Some(hit) = world.hit(ray, 0.0, 10.0) {
        return 0.5 * Colour::new(hit.normal.x + 1.0, hit.normal.y + 1.0, hit.normal.z + 1.0);
    }
    let unit_direction = ray.direction.unit_vector();
    let t = 0.5 * (unit_direction.y + 1.0);
    (1.0 - t) * Colour::new(1, 1, 1) + t * Colour::new(0.5, 0.7, 1.0)
}

fn write_pixel<T: Write>(output: &mut T, c: Colour) -> Result<()> {
    let r = (255.999 * c.x).floor().abs() as u32;
    let g = (255.999 * c.y).floor().abs() as u32;
    let b = (255.999 * c.z).floor().abs() as u32;
    output
        .write(format!("{} {} {}\n", r, g, b).as_bytes())
        .and(Ok(()))
        .context(format!("Writing pixel {}", c))
}
