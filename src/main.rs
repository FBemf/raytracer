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
mod textures;

use camera::Camera;
use hitting::{cast_ray, BVHNode, Colour, Hittable, Material};
use materials::{Dielectric, DiffuseLight, Lambertian, Metal};
use math::{clamp, coeff, dot, Point3, Ray, Vec3};
use objects::{Block, MovingSphere, RotateY, Sphere, Translate, XYRect, XZRect, YZRect};
use progress::{Progress, TimedProgressBar};
use textures::{Checkered, SolidColour};

#[derive(Debug, StructOpt)]
#[structopt(name = "raytracer", about = "Raytracing in a weekend!")]
struct Opt {
    /// Output file
    #[structopt(parse(from_os_str))]
    file: PathBuf,
    /// Output image width
    #[structopt(short, long, default_value = "600")]
    width: u32,
    /// Rays per pixel
    #[structopt(short = "s", long, default_value = "100")]
    ray_samples: u32,
}

type Sky = Box<dyn Fn(&Ray) -> Colour + Send + Sync + 'static>;

fn main() -> Result<()> {
    // cli args
    let opt = Opt::from_args();

    // Output streams
    let mut info = io::stderr();

    // Camera & World
    let (camera, world, sky, aspect_ratio) = _cornell_box();
    //let (camera, world, sky, aspect_ratio) = _random_scene();

    // Image
    let image_width = opt.width;
    let image_height = (image_width as f64 / aspect_ratio).round() as u32;

    // UI
    let progress_bar_len = 60;
    let render_start = Instant::now();

    let samples_per_pixel = opt.ray_samples;
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

    done_receiver.recv()??;

    let img: RgbImage = ImageBuffer::from_raw(image_width, image_height, pixels).unwrap();
    img.save(opt.file)?;

    let elapsed = render_start.elapsed().as_secs();
    eprintln!("Completed in {}:{:02}", elapsed / 60, elapsed % 60,);

    Ok(())
}

fn colour_to_raw(c: Colour) -> Vec<u8> {
    let r = (255.0 * clamp(c.x.abs(), 0.0, 0.999)).floor() as u8;
    let g = (255.0 * clamp(c.y.abs(), 0.0, 0.999)).floor() as u8;
    let b = (255.0 * clamp(c.z.abs(), 0.0, 0.999)).floor() as u8;
    vec![r, g, b]
}

fn _random_scene() -> (Camera, Box<dyn Hittable>, Sky, f64) {
    //Camera
    let look_from = Point3::new(13, 2, 3);
    let look_at = Point3::new(0, 0, 0);
    let direction_up = Vec3::new(0, 1, 0);
    let field_of_view = 20;
    let aspect_ratio = 3.0 / 2.0;
    let aperture = 0.1;
    let distance_to_focus = 10.0;
    let start_time = 0.0;
    let end_time = 1.0;
    let camera = Camera::new(
        look_from,
        look_at,
        direction_up,
        field_of_view,
        aspect_ratio,
        aperture,
        distance_to_focus,
        start_time,
        end_time,
    );

    // Materials
    let material_ground: Arc<dyn Material> = Lambertian::with_texture(Arc::new(Checkered {
        odd: Arc::new(SolidColour {
            colour: Colour::new(0.2, 0.3, 0.1),
        }),
        even: Arc::new(SolidColour {
            colour: Colour::new(0.9, 0.9, 0.9),
        }),
        tile_size: 10.0,
    }));
    let material_glass: Arc<dyn Material> = Arc::new(Dielectric {
        index_of_refraction: 1.5,
    });
    let material_matte: Arc<dyn Material> = Lambertian::with_colour(Colour::new(0.4, 0.2, 0.1));
    let material_light: Arc<dyn Material> = Arc::new(DiffuseLight {
        emit: Arc::new(SolidColour {
            colour: Colour::new(15, 15, 15),
        }),
    });
    let material_metal: Arc<dyn Material> = Arc::new(Metal {
        albedo: Colour::new(0.7, 0.6, 0.5),
        fuzz: 0.0,
    });

    // World
    let mut world = Vec::new();

    world.push(Sphere::new(
        Point3::new(0, -1000, 0),
        1000.0,
        &material_ground,
    ));
    world.push(Sphere::new(Point3::new(0, 1, 0), 1.0, &material_glass));
    world.push(Sphere::new(Point3::new(-4, 1, 0), 1.0, &material_matte));
    world.push(Sphere::new(Point3::new(4, 8, 3), 2.0, &material_light));
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
                    if choose_mat < 0.6 {
                        let albedo = coeff(_random_colour(0, 1), _random_colour(0, 1));
                        //let centre2 = centre + Vec3::new(0.0, rng.gen_range(0.0..0.5), 0.0);
                        let material: Arc<dyn Material> = Lambertian::with_colour(albedo);
                        //world.push(MovingSphere::new(centre, centre2, 0.0, 1.0, 0.2, &material));
                        world.push(Sphere::new(centre, 0.2, &material));
                    } else if choose_mat < 0.9 {
                        let albedo = _random_colour(0.5, 1.0);
                        let fuzz = rng.gen_range(0.0..0.5);
                        let material: Arc<dyn Material> = Arc::new(Metal { albedo, fuzz });
                        world.push(Sphere::new(centre, 0.2, &material));
                    } else {
                        world.push(Sphere::new(centre, 0.2, &Arc::clone(&material_glass)));
                    };
                }
            }
        }
    }
    let world = BVHNode::from_vec(world, start_time, end_time);

    //(camera, world, Box::new(light_background))
    (
        camera,
        world,
        Box::new(|_| Colour::new(0.02, 0.02, 0.02)),
        aspect_ratio,
    )
}

fn _cornell_box() -> (Camera, Box<dyn Hittable>, Sky, f64) {
    //Camera
    let look_from = Point3::new(278, 278, -800);
    let look_at = Point3::new(278, 278, 0);
    let direction_up = Vec3::new(0, 1, 0);
    //let direction_up = Vec3::new(1, 0, -1);
    let field_of_view = 40;
    let aspect_ratio = 1.0;
    //let aperture = 0.1;
    let aperture = 0.0;
    let distance_to_focus = 1000.0;
    let start_time = 0.0;
    let end_time = 1.0;
    let camera = Camera::new(
        look_from,
        look_at,
        direction_up,
        field_of_view,
        aspect_ratio,
        aperture,
        distance_to_focus,
        start_time,
        end_time,
    );

    // Materials
    let red = Lambertian::with_colour(Colour::new(0.65, 0.05, 0.05));
    let white = Lambertian::with_colour(Colour::new(0.73, 0.73, 0.73));
    let green = Lambertian::with_colour(Colour::new(0.12, 0.45, 0.15));
    let metal: Arc<dyn Material> = Arc::new(Metal {
        albedo: Colour::new(0.8, 0.8, 0.8),
        fuzz: 0.0,
    });
    let light: Arc<dyn Material> = Arc::new(DiffuseLight {
        emit: Arc::new(SolidColour {
            colour: Colour::new(15, 15, 15),
        }),
    });
    let glass: Arc<dyn Material> = Arc::new(Dielectric {
        index_of_refraction: 1.5,
    });

    let block1: Arc<dyn Hittable> =
        Block::new(Point3::new(-82, 0, -82), Point3::new(82, 330, 82), &white).into();
    let block2: Arc<dyn Hittable> =
        Block::new(Point3::new(-90, 0, -90), Point3::new(90, 180, 90), &glass).into();
    let block3: Arc<dyn Hittable> =
        Block::new(Point3::new(-90, 0, -90), Point3::new(90, 180, 90), &white).into();

    // World
    let world = vec![
        YZRect::new(0, 555, 0, 555, 555, &green, false),
        YZRect::new(0, 555, 0, 555, 0, &red, true),
        XZRect::new(213, 343, 227, 332, 554, &light, false),
        XZRect::new(0, 555, 0, 555, 555, &white, true),
        XZRect::new(0, 555, 0, 555, 0, &white, true),
        XYRect::new(0, 555, 0, 555, 555, &white, false),
        //Translate::translate(
        //    &RotateY::by_degrees(&block1, -15.0).into(),
        //    Vec3::new(150, 0.1, 360),
        //),
        Sphere::new(Point3::new(155, 100, 220), 100.0, &glass),
        Sphere::new(Point3::new(155, 100, 220), 85.0, &white),
        Translate::translate(
            &RotateY::by_degrees(&block2, -31.0).into(),
            Vec3::new(377, 0.1, 377),
        ),
        Translate::translate(
            &RotateY::by_degrees(&block3, -68.0).into(),
            Vec3::new(377, 180.2, 377),
        ),
    ];
    //let world = BVHNode::from_vec(world, start_time, end_time);
    let world = Box::new(world);

    (
        camera,
        world,
        Box::new(|_| Colour::new(0, 0, 0)),
        aspect_ratio,
    )
}

fn _random_colour<T: Into<f64>>(low: T, high: T) -> Colour {
    let mut rng = rand::thread_rng();
    let low: f64 = low.into();
    let high: f64 = high.into();
    Colour {
        x: rng.gen_range(low..=high),
        y: rng.gen_range(low..=high),
        z: rng.gen_range(low..=high),
    }
}

fn _gradient_background(ray: &Ray, dir: Vec3, col1: Colour, col2: Colour) -> Colour {
    // col1 used to be 1,1,1, col2 used to be 0.5,0.7,1.0
    let gradient_pos = dot(dir, ray.direction.unit_vector());
    let t = 0.5 * (gradient_pos + 1.0);
    1.0 * ((1.0 - t) * col1 + t * col2)
}
