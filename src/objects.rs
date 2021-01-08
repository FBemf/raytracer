use std::sync::Arc;

use crate::camera::{TIME_MAX, TIME_MIN};
use crate::hitting::{surrounding_box, HitRecord, Hittable, Material, AABB};
use crate::math::{distance_to_sphere, get_sphere_uv, Point3, Ray, Vec3};

pub struct Sphere {
    centre: Point3,
    radius: f64,
    material: Arc<dyn Material>,
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
}

pub struct MovingSphere {
    centre0: Point3,
    centre1: Point3,
    time0: f64,
    time1: f64,
    radius: f64,
    material: Arc<dyn Material>,
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
}

pub struct Block {
    minimum: Point3,
    maximum: Point3,
    sides: Vec<Box<dyn Hittable>>,
}

impl Block {
    pub fn new(
        minimum: Point3,
        maximum: Point3,
        material: &Arc<dyn Material>,
    ) -> Box<dyn Hittable> {
        let sides = vec![
            XYRect::new(
                minimum.x, maximum.x, minimum.y, maximum.y, minimum.z, material, true,
            ),
            XYRect::new(
                minimum.x, maximum.x, minimum.y, maximum.y, maximum.z, material, false,
            ),
            XZRect::new(
                minimum.x, maximum.x, minimum.z, maximum.z, minimum.y, material, true,
            ),
            XZRect::new(
                minimum.x, maximum.x, minimum.z, maximum.z, maximum.y, material, false,
            ),
            YZRect::new(
                minimum.y, maximum.y, minimum.z, maximum.z, minimum.x, material, true,
            ),
            YZRect::new(
                minimum.y, maximum.y, minimum.z, maximum.z, maximum.x, material, false,
            ),
        ];
        Box::new(Block {
            minimum,
            maximum,
            sides,
        })
    }
}

impl Hittable for Block {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        self.sides.hit(ray, min_dist, max_dist)
    }
    fn bounding_box(&self, _time0: f64, _time1: f64) -> Option<AABB> {
        Some(AABB {
            minimum: self.minimum,
            maximum: self.maximum,
        })
    }
}

pub struct XYRect {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    k: f64,
    material: Arc<dyn Material>,
    facing_positive: bool,
}

impl XYRect {
    pub fn new<T: Into<f64>>(
        x0: T,
        x1: T,
        y0: T,
        y1: T,
        k: T,
        material: &Arc<dyn Material>,
        facing_positive: bool,
    ) -> Box<dyn Hittable> {
        Box::new(XYRect {
            x0: x0.into(),
            x1: x1.into(),
            y0: y0.into(),
            y1: y1.into(),
            k: k.into(),
            material: Arc::clone(material),
            facing_positive,
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
            Vec3::new(0, 0, if self.facing_positive { 1 } else { -1 }),
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
}

pub struct XZRect {
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    k: f64,
    material: Arc<dyn Material>,
    facing_positive: bool,
}

impl XZRect {
    pub fn new<T: Into<f64>>(
        x0: T,
        x1: T,
        z0: T,
        z1: T,
        k: T,
        material: &Arc<dyn Material>,
        facing_positive: bool,
    ) -> Box<dyn Hittable> {
        Box::new(XZRect {
            x0: x0.into(),
            x1: x1.into(),
            z0: z0.into(),
            z1: z1.into(),
            k: k.into(),
            material: Arc::clone(material),
            facing_positive,
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
            Vec3::new(0, if self.facing_positive { 1 } else { -1 }, 0),
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
}

pub struct YZRect {
    y0: f64,
    y1: f64,
    z0: f64,
    z1: f64,
    k: f64,
    material: Arc<dyn Material>,
    facing_positive: bool,
}

impl YZRect {
    pub fn new<T: Into<f64>>(
        y0: T,
        y1: T,
        z0: T,
        z1: T,
        k: T,
        material: &Arc<dyn Material>,
        facing_positive: bool,
    ) -> Box<dyn Hittable> {
        Box::new(YZRect {
            y0: y0.into(),
            y1: y1.into(),
            z0: z0.into(),
            z1: z1.into(),
            k: k.into(),
            material: Arc::clone(material),
            facing_positive,
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
            Vec3::new(if self.facing_positive { 1 } else { -1 }, 0, 0),
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
}

pub struct Translate {
    original: Arc<dyn Hittable>,
    offset: Vec3,
}

impl Translate {
    pub fn translate(target: &Arc<dyn Hittable>, offset: Vec3) -> Box<dyn Hittable> {
        Box::new(Translate {
            original: Arc::clone(target),
            offset,
        })
    }
}

impl Hittable for Translate {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        let moved_ray = Ray {
            origin: ray.origin - self.offset,
            direction: ray.direction,
            time: ray.time,
        };
        if let Some(hit) = self.original.hit(&moved_ray, min_dist, max_dist) {
            Some(HitRecord {
                distance: hit.distance,
                intersection: hit.intersection + self.offset,
                front_face: hit.front_face,
                material: hit.material,
                normal: hit.normal,
                surface_u: hit.surface_u,
                surface_v: hit.surface_v,
            })
        } else {
            None
        }
    }
    fn bounding_box(&self, time0: f64, time1: f64) -> Option<AABB> {
        if let Some(bb) = self.original.bounding_box(time0, time1) {
            Some(AABB {
                minimum: bb.minimum + self.offset,
                maximum: bb.maximum + self.offset,
            })
        } else {
            None
        }
    }
}

pub struct RotateY {
    original: Arc<dyn Hittable>,
    sin_theta: f64,
    cos_theta: f64,
    bbox: Option<AABB>,
}

impl RotateY {
    pub fn by_degrees(original: &Arc<dyn Hittable>, degrees: f64) -> Box<dyn Hittable> {
        Self::by_radians(original, degrees.to_radians())
    }
    pub fn by_radians(original: &Arc<dyn Hittable>, radians: f64) -> Box<dyn Hittable> {
        let sin_theta = radians.sin();
        let cos_theta = radians.cos();
        let bounding_box = if let Some(bbox) = original.bounding_box(TIME_MIN, TIME_MAX) {
            let mut minimum = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
            let mut maximum = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
            for i in 0..3 {
                for j in 0..3 {
                    for k in 0..3 {
                        let x = i as f64 * bbox.maximum.x + (1.0 - i as f64) * bbox.minimum.x;
                        let y = j as f64 * bbox.maximum.y + (1.0 - j as f64) * bbox.minimum.y;
                        let z = k as f64 * bbox.maximum.z + (1.0 - k as f64) * bbox.minimum.z;

                        let new_x = cos_theta * x + sin_theta * z;
                        let new_z = -sin_theta * x + cos_theta * z;

                        let tester = Vec3::new(new_x, y, new_z);

                        for c in 0..3 {
                            minimum[c] = f64::min(minimum[c], tester[c]);
                            maximum[c] = f64::max(maximum[c], tester[c]);
                        }
                    }
                }
            }
            Some(AABB { minimum, maximum })
        } else {
            None
        };
        Box::new(RotateY {
            original: Arc::clone(original),
            sin_theta,
            cos_theta,
            bbox: bounding_box,
        })
    }
}

impl Hittable for RotateY {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        let x = self.cos_theta * ray.origin.x - self.sin_theta * ray.origin.z;
        let z = self.sin_theta * ray.origin.x + self.cos_theta * ray.origin.z;
        let origin = Vec3::new(x, ray.origin.y, z);

        let x = self.cos_theta * ray.direction.x - self.sin_theta * ray.direction.z;
        let z = self.sin_theta * ray.direction.x + self.cos_theta * ray.direction.z;
        let direction = Vec3::new(x, ray.direction.y, z);

        let rotated = Ray {
            origin,
            direction,
            time: ray.time,
        };

        if let Some(hit) = self.original.hit(&rotated, min_dist, max_dist) {
            let x = self.cos_theta * hit.intersection.x + self.sin_theta * hit.intersection.z;
            let z = -self.sin_theta * hit.intersection.x + self.cos_theta * hit.intersection.z;
            let intersection = Point3::new(x, hit.intersection.y, z);

            let x = self.cos_theta * hit.normal.x + self.sin_theta * hit.normal.z;
            let z = -self.sin_theta * hit.normal.x + self.cos_theta * hit.normal.z;
            let normal = Point3::new(x, hit.normal.y, z);

            Some(HitRecord {
                distance: hit.distance,
                intersection,
                front_face: hit.front_face,
                material: hit.material,
                normal,
                surface_u: hit.surface_u,
                surface_v: hit.surface_v,
            })
        } else {
            None
        }
    }
    fn bounding_box(&self, _time0: f64, _time1: f64) -> Option<AABB> {
        self.bbox
    }
}
