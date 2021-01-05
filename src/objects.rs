use std::sync::Arc;

use crate::hitting::{HitRecord, Hittable, Material};
use crate::math::{Point3, Ray};

pub struct Sphere {
    pub centre: Point3,
    pub radius: f64,
    pub material: Arc<dyn Material>,
}

impl Sphere {
    pub fn new(centre: Point3, radius: f64, material: &Arc<dyn Material>) -> Box<dyn Hittable> {
        Box::new(Sphere {
            centre,
            radius,
            material: Arc::clone(material),
        })
    }
}

impl Hittable for Sphere {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        let oc = ray.origin - self.centre;
        let a = ray.direction.length_squared();
        let half_b = oc * ray.direction;
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = half_b * half_b - a * c;

        if discriminant < 0.0 {
            return None;
        }

        let sqrt_d = discriminant.sqrt();
        let mut root_distance = (-half_b - sqrt_d) / a;

        // find nearest root within
        if root_distance < min_dist || root_distance > max_dist {
            root_distance = (-half_b + sqrt_d) / a;
            if root_distance < min_dist || root_distance > max_dist {
                return None;
            }
        }

        let outward_normal = (ray.at(root_distance) - self.centre) / self.radius;
        Some(HitRecord::new(
            ray,
            root_distance,
            outward_normal,
            Arc::clone(&self.material),
        ))
    }
}
