use rand::Rng;

use std::sync::Arc;

use crate::hitting::{surrounding_box, Colour, HitRecord, Hittable, Material, AABB};
use crate::materials::{DiffuseLight, Lambertian};
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

pub struct ConstantMedium {
    boundary: Arc<dyn Hittable>,
    phase_function: Arc<dyn Material>,
    neg_inv_density: f64,
}

impl ConstantMedium {
    pub fn new(
        boundary: &Arc<dyn Hittable>,
        phase_function: &Arc<dyn Material>,
        density: f64,
    ) -> Box<dyn Hittable> {
        Box::new(ConstantMedium {
            boundary: Arc::clone(boundary),
            phase_function: Arc::clone(phase_function),
            neg_inv_density: -1.0 / density,
        })
    }
}

impl Hittable for ConstantMedium {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        if let Some(mut hit1) = self.boundary.hit(ray, f64::NEG_INFINITY, f64::INFINITY) {
            if let Some(mut hit2) = self
                .boundary
                .hit(ray, hit1.distance + 0.0001, f64::INFINITY)
            {
                if hit1.distance < min_dist {
                    hit1.distance = min_dist;
                }
                if hit2.distance > max_dist {
                    hit2.distance = max_dist;
                }
                if hit1.distance >= hit2.distance {
                    None
                } else {
                    if hit1.distance < 0.0 {
                        hit1.distance = 0.0;
                    }
                    let ray_length = ray.direction.length();
                    let distance_inside_boundary = (hit2.distance - hit1.distance) * ray_length;
                    let hit_distance = self.neg_inv_density
                        * rand::thread_rng().gen_range::<f64, _>(0.0..1.0).ln();
                    if hit_distance > distance_inside_boundary {
                        None
                    } else {
                        let distance = hit1.distance + hit_distance / ray_length;
                        Some(HitRecord {
                            distance,
                            intersection: ray.at(distance),
                            normal: Vec3::new(1, 0, 0), // arbitrary.
                            front_face: true,           // also arbitrary.
                            material: Arc::clone(&self.phase_function),
                            surface_u: 0.0, // (u, v) is meaningless here
                            surface_v: 0.0, //
                        })
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    fn bounding_box(&self, time0: f64, time1: f64) -> Option<AABB> {
        self.boundary.bounding_box(time0, time1)
    }
}

pub struct Spotlight {
    minimum: Point3,
    maximum: Point3,
    panes: Vec<Box<dyn Hittable>>,
}

impl Spotlight {
    pub fn new(minimum: Point3, maximum: Point3, light: Colour) -> Box<dyn Hittable> {
        let dark = Lambertian::with_colour(Colour::new(0, 0, 0));
        let light = DiffuseLight::with_colour(light);
        let panes = vec![
            XYRect::new(
                minimum.x, maximum.x, minimum.y, maximum.y, minimum.z, &dark, true,
            ),
            XYRect::new(
                minimum.x, maximum.x, minimum.y, maximum.y, maximum.z, &dark, true,
            ),
            XZRect::new(
                minimum.x, maximum.x, minimum.z, maximum.z, maximum.y, &light, false,
            ),
            YZRect::new(
                minimum.y, maximum.y, minimum.z, maximum.z, minimum.x, &dark, true,
            ),
            YZRect::new(
                minimum.y, maximum.y, minimum.z, maximum.z, maximum.x, &dark, false,
            ),
        ];
        Box::new(Spotlight {
            minimum,
            maximum,
            panes,
        })
    }
}

impl Hittable for Spotlight {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        self.panes.hit(ray, min_dist, max_dist)
    }
    fn bounding_box(&self, _time0: f64, _time1: f64) -> Option<AABB> {
        Some(AABB {
            minimum: self.minimum,
            maximum: self.maximum,
        })
    }
}
