use std::f64::consts::PI;
use std::sync::Arc;

use crate::hitting::{surrounding_box, HitRecord, Hittable, Material, AABB};
use crate::math::{dot, Point3, Ray, Vec3};

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
        let root_distance = if let Some(d) =
            distance_to_sphere(ray, self.centre, self.radius, min_dist, max_dist)
        {
            d
        } else {
            return None;
        };
        let outward_normal = (ray.at(root_distance) - self.centre) / self.radius;
        Some(HitRecord::new(
            ray,
            root_distance,
            outward_normal,
            Arc::clone(&self.material),
            get_sphere_uv(outward_normal),
        ))
    }
    fn bounding_box(&self, _time0: f64, _time1: f64) -> Option<AABB> {
        Some(AABB {
            minimum: self.centre - Vec3::new(self.radius, self.radius, self.radius),
            maximum: self.centre + Vec3::new(self.radius, self.radius, self.radius),
        })
    }
    fn print(&self, indent: usize) -> String {
        format!(
            "{}Sphere with centre {} and radius {}. bbox {}\n",
            " ".repeat(indent),
            self.centre,
            self.radius,
            self.bounding_box(0.0, 0.0).unwrap().print()
        )
    }
}

pub struct MovingSphere {
    pub centre0: Point3,
    pub centre1: Point3,
    pub time0: f64,
    pub time1: f64,
    pub radius: f64,
    pub material: Arc<dyn Material>,
}

impl MovingSphere {
    pub fn new(
        centre0: Point3,
        centre1: Point3,
        time0: f64,
        time1: f64,
        radius: f64,
        material: &Arc<dyn Material>,
    ) -> Box<dyn Hittable> {
        Box::new(MovingSphere {
            centre0,
            centre1,
            time0,
            time1,
            radius,
            material: Arc::clone(material),
        })
    }

    fn centre(&self, time: f64) -> Point3 {
        self.centre0
            + ((time - self.time0) / (self.time1 - self.time0)) * (self.centre1 - self.centre0)
    }
}

impl Hittable for MovingSphere {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        let centre = self.centre(ray.time);
        let root_distance =
            if let Some(d) = distance_to_sphere(ray, centre, self.radius, min_dist, max_dist) {
                d
            } else {
                return None;
            };
        let outward_normal = (ray.at(root_distance) - centre) / self.radius;
        Some(HitRecord::new(
            ray,
            root_distance,
            outward_normal,
            Arc::clone(&self.material),
            get_sphere_uv(outward_normal),
        ))
    }
    fn bounding_box(&self, time0: f64, time1: f64) -> Option<AABB> {
        let box0 = AABB {
            minimum: self.centre(time0) - Vec3::new(self.radius, self.radius, self.radius),
            maximum: self.centre(time0) + Vec3::new(self.radius, self.radius, self.radius),
        };
        let box1 = AABB {
            minimum: self.centre(time1) - Vec3::new(self.radius, self.radius, self.radius),
            maximum: self.centre(time1) + Vec3::new(self.radius, self.radius, self.radius),
        };
        Some(surrounding_box(&box0, &box1))
    }
    fn print(&self, indent: usize) -> String {
        format!(
            "{}Moving Sphere with centre0 {}, centre1 {}, and radius {}. bbox {}\n",
            " ".repeat(indent),
            self.centre0,
            self.centre1,
            self.radius,
            self.bounding_box(self.time0, self.time1).unwrap().print(),
        )
    }
}

fn distance_to_sphere(
    ray: &Ray,
    centre: Point3,
    radius: f64,
    min_dist: f64,
    max_dist: f64,
) -> Option<f64> {
    let oc = ray.origin - centre;
    let a = ray.direction.length_squared();
    let half_b = dot(oc, ray.direction);
    let c = oc.length_squared() - radius * radius;
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
    Some(root_distance)
}

fn get_sphere_uv(p: Point3) -> (f64, f64) {
    let theta = (-p.y).acos();
    let phi = (-p.z).atan2(p.x) + PI;
    let u = phi / (2.0 * PI);
    let v = theta / PI;
    (u, v)
}

pub struct XYRect {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    k: f64,
    material: Arc<dyn Material>,
}

impl XYRect {
    pub fn new<T: Into<f64>>(
        x0: T,
        x1: T,
        y0: T,
        y1: T,
        k: T,
        material: &Arc<dyn Material>,
    ) -> Box<dyn Hittable> {
        Box::new(XYRect {
            x0: x0.into(),
            x1: x1.into(),
            y0: y0.into(),
            y1: y1.into(),
            k: k.into(),
            material: Arc::clone(material),
        })
    }
}

impl Hittable for XYRect {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        let t = (self.k - ray.origin.z) / ray.direction.z;
        if t < min_dist || t > max_dist {
            return None;
        }
        let x = ray.origin.x + t * ray.direction.x;
        let y = ray.origin.y + t * ray.direction.y;
        if x < self.x0 || x > self.x1 || y < self.y0 || y > self.y1 {
            return None;
        }
        let u = (x - self.x0) / (self.x1 - self.x0);
        let v = (y - self.y0) / (self.y1 - self.y0);
        Some(HitRecord::new(
            ray,
            t,
            Vec3::new(0, 0, 1),
            Arc::clone(&self.material),
            (u, v),
        ))
    }
    fn bounding_box(&self, _time0: f64, _time1: f64) -> Option<AABB> {
        Some(AABB {
            minimum: Point3::new(self.x0, self.y0, self.k - 0.0001),
            maximum: Point3::new(self.x1, self.y1, self.k + 0.0001),
        })
    }
    fn print(&self, _indent: usize) -> String {
        format!(
            "XYBox with x={}-{}, y={}-{}, z={}",
            self.x0, self.x1, self.y0, self.y1, self.k
        )
    }
}

pub struct XZRect {
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    k: f64,
    material: Arc<dyn Material>,
}

impl XZRect {
    pub fn new<T: Into<f64>>(
        x0: T,
        x1: T,
        z0: T,
        z1: T,
        k: T,
        material: &Arc<dyn Material>,
    ) -> Box<dyn Hittable> {
        Box::new(XZRect {
            x0: x0.into(),
            x1: x1.into(),
            z0: z0.into(),
            z1: z1.into(),
            k: k.into(),
            material: Arc::clone(material),
        })
    }
}

impl Hittable for XZRect {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        let t = (self.k - ray.origin.y) / ray.direction.y;
        if t < min_dist || t > max_dist {
            return None;
        }
        let x = ray.origin.x + t * ray.direction.x;
        let z = ray.origin.z + t * ray.direction.z;
        if x < self.x0 || x > self.x1 || z < self.z0 || z > self.z1 {
            return None;
        }
        let u = (x - self.x0) / (self.x1 - self.x0);
        let v = (z - self.z0) / (self.z1 - self.z0);
        Some(HitRecord::new(
            ray,
            t,
            Vec3::new(0, 1, 0),
            Arc::clone(&self.material),
            (u, v),
        ))
    }
    fn bounding_box(&self, _time0: f64, _time1: f64) -> Option<AABB> {
        Some(AABB {
            minimum: Point3::new(self.x0, self.k - 0.0001, self.z0),
            maximum: Point3::new(self.x1, self.k + 0.0001, self.z1),
        })
    }
    fn print(&self, _indent: usize) -> String {
        format!(
            "XZBox with x={}-{}, y={}, z={}-{}",
            self.x0, self.x1, self.k, self.z0, self.z1
        )
    }
}

pub struct YZRect {
    y0: f64,
    y1: f64,
    z0: f64,
    z1: f64,
    k: f64,
    material: Arc<dyn Material>,
}

impl YZRect {
    pub fn new<T: Into<f64>>(
        y0: T,
        y1: T,
        z0: T,
        z1: T,
        k: T,
        material: &Arc<dyn Material>,
    ) -> Box<dyn Hittable> {
        Box::new(YZRect {
            y0: y0.into(),
            y1: y1.into(),
            z0: z0.into(),
            z1: z1.into(),
            k: k.into(),
            material: Arc::clone(material),
        })
    }
}

impl Hittable for YZRect {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        let t = (self.k - ray.origin.x) / ray.direction.x;
        if t < min_dist || t > max_dist {
            return None;
        }
        let y = ray.origin.y + t * ray.direction.y;
        let z = ray.origin.z + t * ray.direction.z;
        if y < self.y0 || y > self.y1 || z < self.z0 || z > self.z1 {
            return None;
        }
        let u = (y - self.y0) / (self.y1 - self.y0);
        let v = (z - self.z0) / (self.z1 - self.z0);
        Some(HitRecord::new(
            ray,
            t,
            Vec3::new(1, 0, 0),
            Arc::clone(&self.material),
            (u, v),
        ))
    }
    fn bounding_box(&self, _time0: f64, _time1: f64) -> Option<AABB> {
        Some(AABB {
            minimum: Point3::new(self.k - 0.0001, self.y0, self.z0),
            maximum: Point3::new(self.k + 0.0001, self.y1, self.z1),
        })
    }
    fn print(&self, _indent: usize) -> String {
        format!(
            "XZBox with x={}, y={}-{}, z={}-{}",
            self.k, self.y0, self.y1, self.z0, self.z1
        )
    }
}
