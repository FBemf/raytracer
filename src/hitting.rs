use rand::Rng;

use std::cmp::Ordering;
use std::fmt;
use std::sync::Arc;

use crate::math::{coeff, dot, Point3, Ray, Vec3};

pub type Colour = Vec3;

pub fn cast_ray<T: Fn(&Ray) -> Colour>(
    ray: &Ray,
    world: &Arc<dyn Hittable>,
    sky: T,
    bounces: u32,
) -> Colour {
    if bounces == 0 {
        return Colour::new(0, 0, 0);
    }
    // min distance is 0.001, to prevent "shadow acne"
    if let Some(hit) = world.hit(ray, 0.001, f64::INFINITY) {
        let emitted = hit.material.emitted(&hit);
        if let Some((new_ray, attenuation)) = hit.material.scatter(ray, &hit) {
            emitted + coeff(attenuation, cast_ray(&new_ray, world, sky, bounces - 1))
        } else {
            emitted
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
    pub surface_u: f64,
    pub surface_v: f64,
    pub material: Arc<dyn Material>,
}

impl fmt::Debug for HitRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HitRecord")
            .field("intersection", &self.intersection)
            .field("normal", &self.normal)
            .field("distance", &self.distance)
            .field("front_face", &self.front_face)
            .field("surface_u", &self.surface_u)
            .field("surface_v", &self.surface_v)
            .finish()
    }
}

impl HitRecord {
    pub fn new(
        ray: &Ray,
        distance: f64,
        outward_normal: Vec3,
        material: Arc<dyn Material>,
        uv: (f64, f64),
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
            surface_u: uv.0,
            surface_v: uv.1,
        }
    }
}

pub trait Hittable: Send + Sync {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord>;
    fn bounding_box(&self, time0: f64, time1: f64) -> Option<AABB>;
    fn _print(&self) -> String;
}

impl Hittable for Vec<Arc<dyn Hittable>> {
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
    fn bounding_box(&self, time0: f64, time1: f64) -> Option<AABB> {
        if self.len() == 0 {
            return None;
        }
        let mut working_box = if let Some(b) = self[0].bounding_box(time0, time1) {
            b
        } else {
            return None;
        };

        for obj in self {
            if let Some(new_box) = obj.bounding_box(time0, time1) {
                working_box = surrounding_box(&new_box, &working_box);
            } else {
                return None;
            }
        }
        Some(working_box)
    }
    fn _print(&self) -> String {
        let mut acc = String::from("vec: [");
        for h in self {
            acc += &(h._print() + ", ");
        }
        acc + "]"
    }
}

pub trait Material: Send + Sync {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> Option<(Ray, Colour)>;
    fn emitted(&self, _hit: &HitRecord) -> Colour {
        Colour::new(0, 0, 0)
    }
    fn _print(&self) -> String;
}

pub struct BVHNode {
    left: Arc<dyn Hittable>,
    right: Arc<dyn Hittable>,
    bbox: AABB,
}

impl BVHNode {
    pub fn from_vec(objects: Vec<Arc<dyn Hittable>>, time0: f64, time1: f64) -> Arc<dyn Hittable> {
        let mut no_bbox: Vec<Arc<dyn Hittable>> = objects
            .iter()
            .filter(|x| x.bounding_box(time0, time1).is_none())
            .map(|x| Arc::clone(x))
            .collect();
        let mut objects: Vec<Arc<dyn Hittable>> = objects
            .into_iter()
            .filter(|x| x.bounding_box(time0, time1).is_some())
            .collect();
        let axis = rand::thread_rng().gen_range(0..3);
        objects.sort_by(|a, b| bbox_compare(a, b, axis));
        if objects.len() == 0 {
            panic!("BVHNode cannot be created from empty slice");
        } else if objects.len() == 1 {
            objects.pop().unwrap()
        } else {
            let halfway = objects.len() / 2;
            let right_objects = objects.split_off(halfway);
            let left_objects = objects;
            let left = Self::from_vec(left_objects, time0, time1);
            let right = Self::from_vec(right_objects, time0, time1);
            let left_bbox = left
                .bounding_box(time0, time1)
                .expect("BHVNode unable to find bbox of subtree");
            let right_bbox = right
                .bounding_box(time0, time1)
                .expect("BHVNode unable to find bbox of subtree");

            let bvh_result: Arc<dyn Hittable> = Arc::new(BVHNode {
                left,
                right,
                bbox: surrounding_box(&left_bbox, &right_bbox),
            });

            if no_bbox.len() == 0 {
                bvh_result
            } else {
                let mut result: Vec<Arc<dyn Hittable>> = vec![bvh_result];
                result.append(&mut no_bbox);
                Arc::new(result)
            }
        }
    }
}

fn bbox_compare(a: &Arc<dyn Hittable>, b: &Arc<dyn Hittable>, axis: usize) -> Ordering {
    a.bounding_box(0.0, 0.0)
        .expect("Unable to find bbox to compare")
        .minimum[axis]
        .partial_cmp(
            &b.bounding_box(0.0, 0.0)
                .expect("Unable to find bbox to compare")
                .minimum[axis],
        )
        .expect("Bounding boxes were incomparable")
}

impl Hittable for BVHNode {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        if !self.bbox.intersects(ray, min_dist, max_dist) {
            None
        } else {
            if let Some(hit_left) = self.left.hit(ray, min_dist, max_dist) {
                if let result_right @ Some(_) =
                    self.right
                        .hit(ray, min_dist, f64::min(hit_left.distance, max_dist))
                {
                    result_right
                } else {
                    Some(hit_left)
                }
            } else {
                self.right.hit(ray, min_dist, max_dist)
            }
        }
    }
    fn bounding_box(&self, _time0: f64, _time1: f64) -> Option<AABB> {
        Some(self.bbox)
    }
    fn _print(&self) -> String {
        format!(
            "bvhNode: ({}), ({})",
            self.left._print(),
            self.right._print()
        )
    }
}

// Axis-aligned bounding box
#[derive(Clone, Copy)]
pub struct AABB {
    pub minimum: Point3,
    pub maximum: Point3,
}

impl AABB {
    pub fn intersects(&self, ray: &Ray, mut min_dist: f64, mut max_dist: f64) -> bool {
        for a in 0..3 {
            let t0 = f64::min(
                (self.minimum[a] - ray.origin[a]) / ray.direction[a],
                (self.maximum[a] - ray.origin[a]) / ray.direction[a],
            );
            let t1 = f64::max(
                (self.minimum[a] - ray.origin[a]) / ray.direction[a],
                (self.maximum[a] - ray.origin[a]) / ray.direction[a],
            );
            min_dist = f64::max(t0, min_dist);
            max_dist = f64::min(t1, max_dist);
            if max_dist <= min_dist {
                return false;
            }
        }
        true
    }
}

pub fn surrounding_box(box0: &AABB, box1: &AABB) -> AABB {
    let minimum = Point3 {
        x: f64::min(box0.minimum.x, box1.minimum.x),
        y: f64::min(box0.minimum.y, box1.minimum.y),
        z: f64::min(box0.minimum.z, box1.minimum.z),
    };
    let maximum = Point3 {
        x: f64::max(box0.maximum.x, box1.maximum.x),
        y: f64::max(box0.maximum.y, box1.maximum.y),
        z: f64::max(box0.maximum.z, box1.maximum.z),
    };
    AABB { minimum, maximum }
}
