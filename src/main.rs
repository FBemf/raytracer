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
mod config;
mod hitting;
mod materials;
mod math;
mod objects;
mod progress;
mod textures;
mod transforms;

use camera::{Camera, Sky};
use config::load_config;
use hitting::{cast_ray, BVHNode, Colour, Hittable, Material};
use materials::{Dielectric, DiffuseLight, Isotropic, Lambertian, Metal};
use math::{clamp, coeff, dot, Point3, Ray, Vec3};
use objects::{Block, ConstantMedium, Sphere, Spotlight, XYRect, XZRect, YZRect};
use progress::{Progress, TimedProgressBar};
use textures::{Checkered, ImageTexture, SolidColour, Texture};
use transforms::{RotateX, RotateY, Translate};

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

fn main() -> Result<()> {
    // cli args
    let opt = Opt::from_args();

    // Output streams
    let mut info = io::stderr();

    // Camera & World
    //let (camera, world, sky, aspect_ratio) = _random_scene();
    //let (camera, world, sky, aspect_ratio) = _cornell_box();
    //let (camera, world, sky, aspect_ratio) = _cornell_smoke();
    //let (camera, world, sky, aspect_ratio) = _globe();
    //let (camera, world, sky, aspect_ratio) = _blocky_scene();
    let (camera, world, sky, aspect_ratio) = load_config("cornell.json5")?;

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

fn _random_scene() -> (Camera, Arc<dyn Hittable>, Sky, f64) {
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

    let checkered: Arc<dyn Texture> = Arc::new(Checkered {
        odd: Arc::new(SolidColour {
            colour: Colour::new(0.2, 0.3, 0.1),
        }),
        even: Arc::new(SolidColour {
            colour: Colour::new(0.9, 0.9, 0.9),
        }),
        tile_size: 10.0,
    });
    // Materials
    let material_ground: Arc<dyn Material> = Lambertian::with_texture(&checkered);
    let material_glass: Arc<dyn Material> = Arc::new(Dielectric {
        index_of_refraction: 1.5,
    });
    let material_matte: Arc<dyn Material> = Lambertian::with_colour(Colour::new(0.4, 0.2, 0.1));
    let material_light: Arc<dyn Material> = Arc::new(DiffuseLight {
        emit: Arc::new(SolidColour {
            colour: Colour::new(20, 15, 7),
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
    //world.push(Sphere::new(Point3::new(4, 8, 3), 2.0, &material_light));
    world.push(Translate::translate(
        &RotateX::by_degrees(
            &XYRect::new(-1, 1, -1, 1, 0, &material_light, false).into(),
            -10.0,
        ),
        Vec3::new(0, 3, 3.5),
    ));
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
        Box::new(|_| Colour::new(0.01, 0.01, 0.01)),
        aspect_ratio,
    )
}

fn _cornell_box() -> (Camera, Arc<dyn Hittable>, Sky, f64) {
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
    let blue = Lambertian::with_colour(Colour::new(0.12, 0.15, 0.45));
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

    let block1 = Block::new(Point3::new(-82, 0, -82), Point3::new(82, 330, 82), &white);
    let block2 = Block::new(Point3::new(-90, 0, -90), Point3::new(90, 180, 90), &glass);
    let block3 = Block::new(Point3::new(-90, 0, -90), Point3::new(90, 180, 90), &white);

    // World
    let world = vec![
        YZRect::new(0, 555, 0, 555, 555, &green, false),
        YZRect::new(0, 555, 0, 555, 0, &red, true),
        XZRect::new(213, 343, 227, 332, 554, &light, false),
        XZRect::new(0, 555, 0, 555, 555, &white, true),
        XZRect::new(0, 555, 0, 555, 0, &white, true),
        XYRect::new(0, 555, 0, 555, 555, &white, false),
        Translate::translate(
            &RotateY::by_degrees(&block1, -15.0),
            Vec3::new(377, 0.1, 377),
        ),
        Sphere::new(Point3::new(155, 100, 320), 100.0, &glass),
        Sphere::new(Point3::new(155, 100, 320), 85.0, &blue),
        Translate::translate(
            &RotateY::by_degrees(&block2, -21.0),
            Vec3::new(307, 0.1, 140),
        ),
        Translate::translate(
            &RotateY::by_degrees(&block2, -78.0),
            Vec3::new(307, 180.2, 140),
        ),
    ];
    let world = BVHNode::from_vec(world, start_time, end_time);
    //let world = Box::new(world);

    (
        camera,
        world,
        Box::new(|_| Colour::new(0, 0, 0)),
        aspect_ratio,
    )
}

fn _cornell_smoke() -> (Camera, Arc<dyn Hittable>, Sky, f64) {
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
    let light: Arc<dyn Material> = Arc::new(DiffuseLight {
        emit: Arc::new(SolidColour {
            colour: Colour::new(7, 7, 7),
        }),
    });
    let smoke: Arc<dyn Material> = Arc::new(Isotropic {
        albedo: Arc::new(SolidColour {
            colour: Colour::new(0, 0, 0),
        }),
    });
    let mist: Arc<dyn Material> = Arc::new(Isotropic {
        albedo: Arc::new(SolidColour {
            colour: Colour::new(1, 1, 1),
        }),
    });

    let block1: Arc<dyn Hittable> =
        Block::new(Point3::new(0, 0, 0), Point3::new(165, 330, 165), &white).into();
    let block2: Arc<dyn Hittable> =
        Block::new(Point3::new(0, 0, 0), Point3::new(165, 165, 165), &white).into();

    // World
    let world = vec![
        YZRect::new(0, 555, 0, 555, 555, &green, false),
        YZRect::new(0, 555, 0, 555, 0, &red, true),
        XZRect::new(113, 443, 127, 432, 554, &light, false),
        XZRect::new(0, 555, 0, 555, 555, &white, true),
        XZRect::new(0, 555, 0, 555, 0, &white, true),
        XYRect::new(0, 555, 0, 555, 555, &white, false),
        ConstantMedium::new(
            &Translate::translate(
                &RotateY::by_degrees(&block1, 15.0).into(),
                Vec3::new(265, 0, 295),
            )
            .into(),
            &smoke,
            0.01,
        ),
        ConstantMedium::new(
            &Translate::translate(
                &RotateY::by_degrees(&block2, -18.0).into(),
                Vec3::new(130, 0, 65),
            )
            .into(),
            &mist,
            0.01,
        ),
    ];
    let world = BVHNode::from_vec(world, start_time, end_time);
    //let world = Box::new(world);

    (
        camera,
        world,
        Box::new(|_| Colour::new(0, 0, 0)),
        aspect_ratio,
    )
}

fn _globe() -> (Camera, Arc<dyn Hittable>, Sky, f64) {
    //Camera
    let look_from = Point3::new(13, 2, 3);
    let look_at = Point3::new(0, 0, 0);
    let direction_up = Vec3::new(0, 1, 0);
    let field_of_view = 90;
    let aspect_ratio = 3.0 / 2.0;
    //let aperture = 0.1;
    let aperture = 0.0;
    let distance_to_focus = 13.0;
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

    let earth_texture = ImageTexture::from_file("za_warudo.jpg").unwrap();
    let earth_material = Lambertian::with_texture(&earth_texture);
    let globe = Sphere::new(Point3::new(0, 0, 0), 2.0, &earth_material);
    let spotlight = Spotlight::new(
        Point3::new(4, -4, 0),
        Point3::new(0, 0, 0),
        4.0,
        2.0,
        Colour::new(100, 100, 100),
    );
    //let spotlight = Spotlight::new_primitive(
    //    Point3::new(-0.5, -0.5, -4),
    //    Point3::new(0.5, 0.5, 4),
    //    Colour::new(10, 10, 10),
    //);
    let world: Arc<dyn Hittable> = Arc::new(vec![globe, spotlight]);
    //let world = globe;

    //let sky = _gradient_background(direction_up, Colour::new(1, 1, 1), Colour::new(0.7, 0.5, 1));
    let sky = _gradient_background(
        direction_up,
        Colour::new(0.03, 0.03, 0.03),
        Colour::new(0.2, 0.02, 0.02),
    );

    (camera, world, sky, aspect_ratio)
}

fn _blocky_scene() -> (Camera, Arc<dyn Hittable>, Sky, f64) {
    //Camera
    let look_from = Point3::new(478, 378, -600);
    let look_at = Point3::new(278, 178, 0);
    let direction_up = Vec3::new(0, 1, 0);
    let field_of_view = 40;
    let aspect_ratio = 16.0 / 9.0;
    let aperture = 0.0;
    let distance_to_focus = 300.0;
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

    let ground = Lambertian::with_colour(Colour::new(0.48, 0.83, 0.53));
    let light = DiffuseLight::with_colour(Colour::new(7, 7, 7));
    let glass: Arc<dyn Material> = Arc::new(Dielectric {
        index_of_refraction: 1.5,
    });
    let mist: Arc<dyn Material> = Arc::new(Isotropic {
        albedo: Arc::new(SolidColour {
            colour: Colour::new(1, 1, 1),
        }),
    });

    let mut rng = rand::thread_rng();
    let mut world = Vec::new();

    for i in 0..20 {
        for j in 0..20 {
            let w = 100.0;
            let x0 = -1000.0 + i as f64 * w;
            let z0 = -1000.0 + j as f64 * w;
            let y0 = -1.0;
            let x1 = x0 + w;
            let y1 = rng.gen_range(0.0..w);
            let z1 = z0 + w;
            world.push(Block::new(
                Point3::new(x0, y0, z0),
                Point3::new(x1, y1, z1),
                &ground,
            ))
        }
    }

    let mut world = vec![BVHNode::from_vec(world, start_time, end_time)];

    //let spotlight: Arc<dyn Hittable> = Spotlight::new_primitive(
    //    Point3::new(-60, 200, -60),
    //    Point3::new(60, 800, 60),
    //    Colour::new(80, 80, 80),
    //)
    //.into();
    //let spotlight = RotateZ::by_degrees(&spotlight, 115.0).into(); // used to be -65 when the spotlight started facing down
    //let spotlight = Translate::translate(&spotlight, Vec3::new(650, 330, 200));
    //world.push(spotlight);
    //world.push(XZRect::new(123, 423, 147, 412, 554, &light, false));
    world.push(Spotlight::new(
        Point3::new(650, 330, 200),
        Point3::new(310, 150, 100),
        600.0,
        120.0,
        Colour::new(80, 80, 80),
    ));
    world.push(Sphere::new(Point3::new(310, 200, 100), 100.0, &glass));
    //world.push(ConstantMedium::new(
    //    &Block::new(
    //        Point3::new(-2000, -2000, -2000),
    //        Point3::new(2000, 2000, 2000),
    //        &ground,
    //    )
    //    .into(),
    //    &mist,
    //    0.0003,
    //));

    //let world = BVHNode::from_vec(world, start_time, end_time);
    let world = Arc::new(world);
    let sky: Sky = Box::new(|_| Colour::new(0, 0, 0));

    (camera, world, sky, aspect_ratio)
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

fn _gradient_background(dir: Vec3, col1: Colour, col2: Colour) -> Sky {
    // col1 used to be 1,1,1, col2 used to be 0.5,0.7,1.0
    let unit_dir = dir.unit_vector();
    Box::new(move |ray: &Ray| {
        let gradient_pos = dot(unit_dir, ray.direction.unit_vector());
        let t = 0.5 * (gradient_pos + 1.0);
        1.0 * ((1.0 - t) * col1 + t * col2)
    })
}
