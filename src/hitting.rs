use crate::math::{Point3, Ray, Vec3};

pub struct HitRecord {
    pub intersection: Point3,
    pub normal: Vec3,
    pub distance: f64,
    pub front_face: bool,
}

impl HitRecord {
    pub fn new(ray: &Ray, distance: f64, outward_normal: Vec3) -> Self {
        let front_face = ray.direction * outward_normal < 0.0;
        let normal = if front_face {
            outward_normal
        } else {
            -outward_normal
        };
        HitRecord {
            intersection: ray.at(distance),
            normal,
            distance,
            front_face,
        }
    }
}

pub trait Hittable {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord>;
}

pub struct Sphere {
    pub centre: Point3,
    pub radius: f64,
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
        let root_distance = (-half_b - sqrt_d) / a;

        // find nearest root within
        if root_distance < min_dist || root_distance > max_dist {
            let root_distance = (-half_b + sqrt_d) / a;
            if root_distance < min_dist || root_distance > max_dist {
                return None;
            }
        }

        let outward_normal = (ray.at(root_distance) - self.centre) / self.radius;
        Some(HitRecord::new(ray, root_distance, outward_normal))
    }
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
