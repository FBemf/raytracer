use std::sync::Arc;

use crate::camera::{TIME_MAX, TIME_MIN};
use crate::hitting::{HitRecord, Hittable, AABB};
use crate::math::{Point3, Ray, Vec3};

pub struct Translate {
    original: Arc<dyn Hittable>,
    offset: Vec3,
}

impl Translate {
    pub fn translate(target: &Arc<dyn Hittable>, offset: Vec3) -> Arc<dyn Hittable> {
        Arc::new(Translate {
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
    fn _print(&self) -> String {
        format!("translate {}", self.original._print())
    }
}

pub struct RotateX {
    original: Arc<dyn Hittable>,
    sin_theta: f64,
    cos_theta: f64,
    bbox: Option<AABB>,
}

impl RotateX {
    pub fn by_degrees(original: &Arc<dyn Hittable>, degrees: f64) -> Arc<dyn Hittable> {
        Self::by_radians(original, degrees.to_radians())
    }
    pub fn by_radians(original: &Arc<dyn Hittable>, radians: f64) -> Arc<dyn Hittable> {
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

                        let new_z = cos_theta * z + sin_theta * y;
                        let new_y = -sin_theta * z + cos_theta * y;

                        let tester = Vec3::new(x, new_y, new_z);

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
        Arc::new(RotateX {
            original: Arc::clone(original),
            sin_theta,
            cos_theta,
            bbox: bounding_box,
        })
    }
}

impl Hittable for RotateX {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        let z = self.cos_theta * ray.origin.z - self.sin_theta * ray.origin.y;
        let y = self.sin_theta * ray.origin.z + self.cos_theta * ray.origin.y;
        let origin = Vec3::new(ray.origin.x, y, z);

        let z = self.cos_theta * ray.direction.z - self.sin_theta * ray.direction.y;
        let y = self.sin_theta * ray.direction.z + self.cos_theta * ray.direction.y;
        let direction = Vec3::new(ray.direction.x, y, z);

        let rotated = Ray {
            origin,
            direction,
            time: ray.time,
        };

        if let Some(hit) = self.original.hit(&rotated, min_dist, max_dist) {
            let z = self.cos_theta * hit.intersection.z + self.sin_theta * hit.intersection.y;
            let y = -self.sin_theta * hit.intersection.z + self.cos_theta * hit.intersection.y;
            let intersection = Point3::new(hit.intersection.x, y, z);

            let z = self.cos_theta * hit.normal.z + self.sin_theta * hit.normal.y;
            let y = -self.sin_theta * hit.normal.z + self.cos_theta * hit.normal.y;
            let normal = Point3::new(hit.normal.x, y, z);

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
    fn _print(&self) -> String {
        format!("rotatex {}", self.original._print())
    }
}

pub struct RotateY {
    original: Arc<dyn Hittable>,
    sin_theta: f64,
    cos_theta: f64,
    bbox: Option<AABB>,
}

impl RotateY {
    pub fn by_degrees(original: &Arc<dyn Hittable>, degrees: f64) -> Arc<dyn Hittable> {
        Self::by_radians(original, degrees.to_radians())
    }
    pub fn by_radians(original: &Arc<dyn Hittable>, radians: f64) -> Arc<dyn Hittable> {
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
        Arc::new(RotateY {
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
    fn _print(&self) -> String {
        format!("rotatey {}", self.original._print())
    }
}

pub struct RotateZ {
    original: Arc<dyn Hittable>,
    sin_theta: f64,
    cos_theta: f64,
    bbox: Option<AABB>,
}

impl RotateZ {
    pub fn by_degrees(original: &Arc<dyn Hittable>, degrees: f64) -> Arc<dyn Hittable> {
        Self::by_radians(original, degrees.to_radians())
    }
    pub fn by_radians(original: &Arc<dyn Hittable>, radians: f64) -> Arc<dyn Hittable> {
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

                        let new_y = cos_theta * y + sin_theta * x;
                        let new_x = -sin_theta * y + cos_theta * x;

                        let tester = Vec3::new(new_x, new_y, z);

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
        Arc::new(RotateZ {
            original: Arc::clone(original),
            sin_theta,
            cos_theta,
            bbox: bounding_box,
        })
    }
}

impl Hittable for RotateZ {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        let y = self.cos_theta * ray.origin.y - self.sin_theta * ray.origin.x;
        let x = self.sin_theta * ray.origin.y + self.cos_theta * ray.origin.x;
        let origin = Vec3::new(x, y, ray.origin.z);

        let y = self.cos_theta * ray.direction.y - self.sin_theta * ray.direction.x;
        let x = self.sin_theta * ray.direction.y + self.cos_theta * ray.direction.x;
        let direction = Vec3::new(x, y, ray.direction.z);

        let rotated = Ray {
            origin,
            direction,
            time: ray.time,
        };

        if let Some(hit) = self.original.hit(&rotated, min_dist, max_dist) {
            let y = self.cos_theta * hit.intersection.y + self.sin_theta * hit.intersection.x;
            let x = -self.sin_theta * hit.intersection.y + self.cos_theta * hit.intersection.x;
            let intersection = Point3::new(x, y, hit.intersection.z);

            let y = self.cos_theta * hit.normal.y + self.sin_theta * hit.normal.x;
            let x = -self.sin_theta * hit.normal.y + self.cos_theta * hit.normal.x;
            let normal = Point3::new(x, y, hit.normal.z);

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
    fn _print(&self) -> String {
        format!("rotatez {}", self.original._print())
    }
}
