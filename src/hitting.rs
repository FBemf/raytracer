use std::sync::Arc;

use crate::math::{Point3, Ray, Vec3};

pub type Colour = Vec3;

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
        let front_face = ray.direction * outward_normal < 0.0;
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
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> Option<(Ray, Colour)>;
}
