use anyhow::{Context, Result};
use std::io::{self, Write};

mod math;
mod progress;

use math::{Point3, Ray, Vec3};
type Colour = Vec3;

use progress::Progress;

fn main() -> Result<()> {
    // Output
    let mut output = io::stdout();
    let mut info = io::stderr();

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
            let colour = ray_colour(&r);
            write_pixel(&mut output, colour)?;
        }
    }
    progress.clear()?;
    eprintln!("Done");

    Ok(())
}

// returns distance to intersection if hit
fn hit_sphere(centre: &Point3, radius: f64, ray: &Ray) -> Option<f64> {
    let oc = ray.origin - *centre;
    let a = ray.direction * ray.direction;
    let b = 2.0 * (oc * ray.direction);
    let c = oc * oc - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant > 0.0 {
        let distance = (-b - discriminant.sqrt()) / (2.0 * a);
        Some(distance)
    } else {
        None
    }
}

fn ray_colour(ray: &Ray) -> Colour {
    let spheres = vec![(0.5, Point3::new(0, 0, -1)), (0.5, Point3::new(1, 1, -2))];
    for (radius, centre) in spheres {
        if let Some(distance) = hit_sphere(&centre, radius, ray) {
            let normal = (ray.at(distance) - centre).unit_vector();
            return 0.5 * Colour::new(normal.x + 1.0, normal.y + 1.0, normal.z + 1.0);
        }
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
