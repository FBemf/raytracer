use rand::Rng;

use std::sync::Arc;

use crate::hitting::{surrounding_box, Colour, HitRecord, Hittable, Material, AABB};
use crate::materials::{DiffuseLight, Lambertian};
use crate::math::{
    cross, distance_to_sphere, dot, get_sphere_uv, line_plane_collision, Point3, Ray, Vec3,
};
use crate::transforms::{RotateY, RotateZ, Translate};

pub struct Sphere {
    centre: Point3,
    radius: f64,
    material: Arc<dyn Material>,
}

impl Sphere {
    pub fn new(centre: Point3, radius: f64, material: &Arc<dyn Material>) -> Arc<dyn Hittable> {
        Arc::new(Sphere {
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
    fn _print(&self) -> String {
        format!(
            "Sphere (centre: {}, radius: {}, material: {})",
            self.centre,
            self.radius,
            self.material._print()
        )
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
    ) -> Arc<dyn Hittable> {
        Arc::new(MovingSphere {
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
    fn _print(&self) -> String {
        format!(
            "Moving Sphere (centre0: {:?}, centre1: {:?}, time0: {}, time1: {}, radius: {}, material: {})",
            self.centre0, self.centre1, self.time0, self.time1, self.radius, self.material._print()
        )
    }
}

pub struct Block {
    minimum: Point3,
    maximum: Point3,
    sides: Vec<Arc<dyn Hittable>>,
}

impl Block {
    pub fn new(
        minimum: Point3,
        maximum: Point3,
        material: &Arc<dyn Material>,
    ) -> Arc<dyn Hittable> {
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
        Arc::new(Block {
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
    fn _print(&self) -> String {
        format!("Block (min: {}, max: {})", self.minimum, self.maximum)
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
    ) -> Arc<dyn Hittable> {
        Arc::new(XYRect {
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
    fn _print(&self) -> String {
        String::from("rect")
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
    ) -> Arc<dyn Hittable> {
        Arc::new(XZRect {
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
    fn _print(&self) -> String {
        String::from("rect")
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
    ) -> Arc<dyn Hittable> {
        Arc::new(YZRect {
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
    fn _print(&self) -> String {
        String::from("rect")
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
    ) -> Arc<dyn Hittable> {
        Arc::new(ConstantMedium {
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
    fn _print(&self) -> String {
        format!(
            "constant medium (boundary: {}, NID: {}, phase function: {})",
            self.boundary._print(),
            self.neg_inv_density,
            self.phase_function._print(),
        )
    }
}

pub struct Triangle {
    point: Point3,
    vec1: Vec3,
    vec2: Vec3,
    normal: Vec3,
    material: Arc<dyn Material>,
}

impl Triangle {
    pub fn new(a: Point3, b: Point3, c: Point3, material: &Arc<dyn Material>) -> Arc<dyn Hittable> {
        let vec1 = b - a;
        let vec2 = c - a;
        let normal = cross(vec1, vec2).unit_vector();
        Arc::new(Triangle {
            point: a,
            vec1,
            vec2,
            normal,
            material: Arc::clone(material),
        })
    }
}

impl Hittable for Triangle {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        if let Some(solution) =
            line_plane_collision(ray.origin, ray.direction, self.point, self.vec1, self.vec2)
        {
            let distance = solution[0];
            if distance < min_dist || distance > max_dist {
                None
            } else if solution[1] < 0.0 || solution[2] < 0.0 || solution[1] + solution[2] > 1.0 {
                None
            } else {
                Some(HitRecord::new(
                    ray,
                    distance,
                    self.normal,
                    Arc::clone(&self.material),
                    (solution[2], solution[1]),
                ))
            }
        } else {
            None
        }
    }
    fn bounding_box(&self, _time0: f64, _time1: f64) -> Option<AABB> {
        None
    }
    fn _print(&self) -> String {
        format!("plane ({}, {}, {})", self.point, self.vec1, self.vec2)
    }
}

pub struct Plane {
    point: Point3,
    vec1: Vec3,
    vec2: Vec3,
    uv_repeat: f64,
    normal: Vec3,
    material: Arc<dyn Material>,
}

impl Plane {
    pub fn new(
        a: Point3,
        b: Point3,
        c: Point3,
        uv_repeat: f64,
        material: &Arc<dyn Material>,
    ) -> Arc<dyn Hittable> {
        let vec1 = (b - a).unit_vector();
        let normal = cross(vec1, c - a).unit_vector();
        let vec2 = cross(normal, vec1);
        Arc::new(Plane {
            point: a,
            vec1,
            vec2,
            uv_repeat,
            normal,
            material: Arc::clone(material),
        })
    }
}

impl Hittable for Plane {
    fn hit(&self, ray: &Ray, min_dist: f64, max_dist: f64) -> Option<HitRecord> {
        if let Some(solution) =
            line_plane_collision(ray.origin, ray.direction, self.point, self.vec1, self.vec2)
        {
            let distance = solution[0];
            if distance < min_dist || distance > max_dist {
                None
            } else {
                let u = (solution[1] % self.uv_repeat) / self.uv_repeat;
                let v = (-solution[2] % self.uv_repeat) / self.uv_repeat;
                let u = if u < 0.0 { 1.0 + u } else { u };
                let v = if v < 0.0 { 1.0 + v } else { v };
                Some(HitRecord::new(
                    ray,
                    distance,
                    self.normal,
                    Arc::clone(&self.material),
                    (u, v),
                ))
            }
        } else {
            None
        }
    }
    fn bounding_box(&self, _time0: f64, _time1: f64) -> Option<AABB> {
        None
    }
    fn _print(&self) -> String {
        format!("plane ({}, {}, {})", self.point, self.vec1, self.vec2)
    }
}

pub struct Spotlight {
    minimum: Point3,
    maximum: Point3,
    panes: Vec<Arc<dyn Hittable>>,
}

impl Spotlight {
    pub fn new_primitive(minimum: Point3, maximum: Point3, light: Colour) -> Arc<dyn Hittable> {
        let dark = Lambertian::with_colour(Colour::new(0, 0, 0));
        //let dark = Lambertian::with_colour(Colour::new(0.3, 0.3, 0.3));
        let light = DiffuseLight::with_colour(light);
        let panes = vec![
            XYRect::new(
                minimum.x, maximum.x, minimum.y, maximum.y, minimum.z, &dark, true,
            ),
            XYRect::new(
                minimum.x, maximum.x, minimum.y, maximum.y, maximum.z, &dark, false,
            ),
            XZRect::new(
                minimum.x, maximum.x, minimum.z, maximum.z, minimum.y, &light, true,
            ),
            YZRect::new(
                minimum.y, maximum.y, minimum.z, maximum.z, minimum.x, &dark, true,
            ),
            YZRect::new(
                minimum.y, maximum.y, minimum.z, maximum.z, maximum.x, &dark, false,
            ),
        ];
        Arc::new(Spotlight {
            minimum,
            maximum,
            panes,
        })
    }
    pub fn new(
        looking_from: Point3,
        looking_at: Point3,
        length: f64,
        width: f64,
        light: Colour,
    ) -> Arc<dyn Hittable> {
        let spotlight = Spotlight::new_primitive(
            Point3::new(-width / 2.0, -length, -width / 2.0),
            Point3::new(width / 2.0, 0, width / 2.0),
            light,
        );
        let direction = looking_at - looking_from;
        let direction_x = Vec3::new(direction.x, 0, 0);
        let direction_z = Vec3::new(0, 0, direction.z);
        // theta1 is the amount to rotate around the z axis
        let theta1 = if direction.length() != 0.0 {
            -dot(Vec3::new(0, 1, 0), direction.unit_vector()).acos()
        } else {
            0.0
        };
        // theta2 is the amount to rotate around the y axis
        let theta2 = if direction_x.x != 0.0 || direction_z.z != 0.0 {
            dot(
                Vec3::new(1, 0, 0),
                (direction_x + direction_z).unit_vector(),
            )
            .acos()
                * -direction_z.z.signum()
        } else {
            0.0
        };
        Translate::translate(
            &RotateY::by_radians(&RotateZ::by_radians(&spotlight, theta1), theta2),
            looking_from,
        )
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
    fn _print(&self) -> String {
        format!("spotlight ({}, {})", self.minimum, self.maximum)
    }
}

#[test]
fn plane_test() {
    let material = Lambertian::with_colour(Colour::new(1, 1, 1));
    let plane = Plane::new(
        Point3::new(0, 0, 0),
        Point3::new(0, 0, 1),
        Point3::new(1, 0, 0),
        5.0,
        &material,
    );
    let r1 = Ray::new(Point3::new(0, 5, 0), Vec3::new(-1.3, -1, 1), 0.0);
    dbg!(plane.hit(&r1, 0.0, f64::INFINITY));
}
