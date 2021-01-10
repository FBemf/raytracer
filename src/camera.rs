use rand::Rng;

use crate::hitting::Colour;
use crate::math::{cross, dot, random_in_unit_disc, Point3, Ray, Vec3};

pub const TIME_MIN: f64 = 0.0;
pub const TIME_MAX: f64 = 1.0;

pub type Sky = Box<dyn Fn(&Ray) -> Colour + Send + Sync + 'static>;

pub struct Camera {
    origin: Point3,
    lower_left_corner: Point3,
    horizontal: Vec3,
    vertical: Vec3,
    u: Vec3,
    v: Vec3,
    _w: Vec3,
    lens_radius: f64,
    start_time: f64,
    end_time: f64,
}

impl Camera {
    pub fn new<T: Into<f64>>(
        look_from: Point3,
        look_at: Point3,
        direction_up: Vec3,
        vertical_fov: T,
        aspect_ratio: f64,
        aperture: f64,
        focus_dist: f64,
        start_time: f64,
        end_time: f64,
    ) -> Camera {
        if start_time < TIME_MIN || end_time > TIME_MAX || start_time > end_time {
            panic!("Camera must have 0 <= start_time <= end_time <= 1");
        }
        let theta = vertical_fov.into().to_radians();
        let viewport_height = 2.0 * (theta / 2.0).tan();
        let viewport_width = aspect_ratio * viewport_height;

        let w = (look_from - look_at).unit_vector();
        let u = cross(direction_up, w).unit_vector();
        let v = cross(w, u);

        let origin = look_from;
        let horizontal = focus_dist * viewport_width * u;
        let vertical = focus_dist * viewport_height * v;
        let lower_left_corner = origin - horizontal / 2.0 - vertical / 2.0 - focus_dist * w;

        let lens_radius = aperture / 2.0;

        Camera {
            origin,
            lower_left_corner,
            horizontal,
            vertical,
            u,
            v,
            _w: w,
            lens_radius,
            start_time,
            end_time,
        }
    }
    pub fn find_ray(&self, s: f64, t: f64) -> Ray {
        let rd = self.lens_radius * random_in_unit_disc();
        let offset = self.u * rd.x + self.v * rd.y;
        Ray {
            origin: self.origin + offset,
            direction: self.lower_left_corner + s * self.horizontal + t * self.vertical
                - self.origin
                - offset,
            time: rand::thread_rng().gen_range(self.start_time..=self.end_time),
        }
    }
}

pub fn gradient_background(dir: Vec3, col1: Colour, col2: Colour) -> Sky {
    // col1 used to be 1,1,1, col2 used to be 0.5,0.7,1.0
    let unit_dir = dir.unit_vector();
    Box::new(move |ray: &Ray| {
        let gradient_pos = dot(unit_dir, ray.direction.unit_vector());
        let t = 0.5 * (gradient_pos + 1.0);
        1.0 * ((1.0 - t) * col1 + t * col2)
    })
}
