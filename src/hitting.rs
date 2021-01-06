use rand::Rng;
use std::sync::Arc;

use crate::math::{coeff, dot, Point3, Ray, Vec3};

pub type Colour = Vec3;

pub fn cast_ray<T: Hittable, H: Fn(&Ray) -> Colour>(
    ray: &Ray,
    world: &T,
    sky: H,
    bounces: u32,
) -> Colour {
    if bounces == 0 {
        return Colour::new(0, 0, 0);
    }
    // min distance is 0.001, to prevent "shadow acne"
    if let Some(hit) = world.hit(ray, 0.001, f64::INFINITY) {
        match hit.material.scatter(ray, &hit) {
            ScatterResult::Scattered(ray, attenuation) => {
                coeff(attenuation, cast_ray(&ray, world, sky, bounces - 1))
            }
            ScatterResult::Emitted(colour) => colour,
        }
    } else {
        sky(ray)
    }
}

pub struct HitRecord {
    pub intersection: Point3,
    pub normal: Vec3,
    pub distance: f64,
    pub front_face: bool,
    pub material: Arc<dyn Material>,
}

impl HitRecord {
    pub fn new(
        ray: &Ray,
        distance: f64,
        outward_normal: Vec3,
        material: Arc<dyn Material>,
    ) -> Self {
        let front_face = dot(ray.direction, outward_normal) < 0.0;
        let normal = if front_face {
            outward_normal.unit_vector()
        } else {
            -outward_normal.unit_vector()
        };
        HitRecord {
            intersection: ray.at(distance),
            normal,
            distance,
            front_face,
            material,
        }
    }
}

pub trait Hittable
where
    Self: Send + Sync,
{
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord>;
}

impl Hittable for Vec<Box<dyn Hittable>> {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        self.iter()
            .map(|x| x.hit(ray, min_dist, max_dist))
            .fold(None, |acc, next| match (acc, next) {
                (Some(old), Some(new)) => {
                    if new.distance < old.distance {
                        Some(new)
                    } else {
                        Some(old)
                    }
                }
                (None, Some(h)) => Some(h),
                (Some(h), None) => Some(h),
                (None, None) => None,
            })
    }
}

pub trait Material
where
    Self: Send + Sync,
{
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> ScatterResult;
}

pub fn random_colour<T: Into<f64>>(low: T, high: T) -> Colour {
    let mut rng = rand::thread_rng();
    let low: f64 = low.into();
    let high: f64 = high.into();
    Colour {
        x: rng.gen_range(low..high),
        y: rng.gen_range(low..high),
        z: rng.gen_range(low..high),
    }
}

pub enum ScatterResult {
    Scattered(Ray, Colour),
    Emitted(Colour),
}
