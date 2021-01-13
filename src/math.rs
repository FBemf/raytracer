use rand::Rng;

use std::f64::consts::PI;
use std::fmt;

pub type Point3 = Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn new<T, U, V>(x: T, y: U, z: V) -> Vec3
    where
        T: Into<f64>,
        U: Into<f64>,
        V: Into<f64>,
    {
        Vec3 {
            x: x.into(),
            y: y.into(),
            z: z.into(),
        }
    }
    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }
    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }
    pub fn unit_vector(&self) -> Vec3 {
        Vec3 {
            x: self.x / self.length(),
            y: self.y / self.length(),
            z: self.z / self.length(),
        }
    }
    pub fn near_zero(&self) -> bool {
        let d = 1e-8;
        self.x.abs() < d && self.y.abs() < d && self.z.abs() < d
    }
}

pub fn cross(lhs: Vec3, rhs: Vec3) -> Vec3 {
    Vec3 {
        x: lhs.y * rhs.z - lhs.z * rhs.y,
        y: lhs.z * rhs.x - lhs.x * rhs.z,
        z: lhs.x * rhs.y - lhs.y * rhs.x,
    }
}

pub fn dot(lhs: Vec3, rhs: Vec3) -> f64 {
    lhs.x * rhs.x + lhs.y * rhs.y + lhs.z * rhs.z
}

pub fn coeff(lhs: Vec3, rhs: Vec3) -> Vec3 {
    Vec3 {
        x: lhs.x * rhs.x,
        y: lhs.y * rhs.y,
        z: lhs.z * rhs.z,
    }
}

pub fn reflect(&direction: &Vec3, &normal: &Vec3) -> Vec3 {
    direction - 2.0 * dot(direction, normal) * normal
}

pub fn refract(&direction: &Vec3, &normal: &Vec3, etai_over_etat: f64) -> Vec3 {
    let direction = direction.unit_vector();
    let cos_theta = f64::min(dot(-direction, normal), 1.0);
    let r_out_perpendicular = etai_over_etat * (direction + cos_theta * normal);
    let r_out_parallel = -((1.0 - r_out_perpendicular.length_squared()).abs().sqrt()) * normal;
    let result = r_out_perpendicular + r_out_parallel;
    result
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl std::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, other: Self) {
        let new = *self + other;
        *self = new;
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl std::ops::SubAssign for Vec3 {
    fn sub_assign(&mut self, other: Self) {
        let new = *self - other;
        *self = new;
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, other: f64) -> Self::Output {
        Vec3 {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
        }
    }
}

impl std::ops::Mul<Vec3> for f64 {
    type Output = Vec3;
    fn mul(self, other: Vec3) -> Self::Output {
        other * self
    }
}

impl std::ops::MulAssign<f64> for Vec3 {
    fn mul_assign(&mut self, other: f64) {
        let new = *self * other;
        *self = new;
    }
}

impl std::ops::Div<f64> for Vec3 {
    type Output = Self;
    fn div(self, other: f64) -> Self::Output {
        Vec3 {
            x: self.x / other,
            y: self.y / other,
            z: self.z / other,
        }
    }
}

impl std::ops::DivAssign<f64> for Vec3 {
    fn div_assign(&mut self, other: f64) {
        let new = *self / other;
        *self = new;
    }
}

impl std::ops::Index<usize> for Vec3 {
    type Output = f64;
    fn index(&self, i: usize) -> &Self::Output {
        match i {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            x => panic!("index {} out of bounds on Vec3", x),
        }
    }
}

impl std::ops::IndexMut<usize> for Vec3 {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        match i {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            x => panic!("index {} out of bounds on Vec3", x),
        }
    }
}

impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

// direction must be a unit vector
#[derive(Debug)]
pub struct Ray {
    pub origin: Point3,
    pub direction: Vec3,
    pub time: f64,
}

impl Ray {
    pub fn new(origin: Point3, direction: Vec3, time: f64) -> Ray {
        Ray {
            origin,
            direction: direction.unit_vector(),
            time,
        }
    }
    pub fn at(&self, t: f64) -> Point3 {
        self.origin + t * self.direction
    }
}

pub fn clamp(a: f64, min: f64, max: f64) -> f64 {
    if a < min {
        min
    } else if a > max {
        max
    } else {
        a
    }
}

pub fn random_unit_vector() -> Vec3 {
    random_in_unit_sphere().unit_vector()
}

pub fn random_in_unit_sphere() -> Vec3 {
    let mut rng = rand::thread_rng();
    loop {
        let p = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        );
        if p.length_squared() <= 1.0 {
            return p;
        }
    }
}

pub fn random_in_unit_disc() -> Vec3 {
    let mut rng = rand::thread_rng();
    loop {
        let p = Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), 0.0);
        if p.length_squared() <= 1.0 {
            return p;
        }
    }
}

pub fn distance_to_sphere(
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

pub fn get_sphere_uv(p: Point3) -> (f64, f64) {
    let theta = (-p.y).acos();
    let phi = (-p.z).atan2(p.x) + PI;
    let u = phi / (2.0 * PI);
    let v = theta / PI;
    (u, v)
}

// collision between a line defined by a point la and a direction lab
// and a plane defined by a point p0 and two directions p01 and p02
pub fn line_plane_collision(
    la: Point3,
    lab: Vec3,
    p0: Point3,
    p01: Vec3,
    p02: Vec3,
) -> Option<Vec3> {
    const MARGIN: f64 = 0.0000001;
    let cross_p = cross(p01, p02);
    let determinant = dot(-lab, cross_p);
    if determinant < -MARGIN || determinant > MARGIN {
        let denominator = dot(-lab, cross_p);
        let t = dot(cross_p, la - p0) / denominator;
        let u = dot(cross(p02, -lab), la - p0) / denominator;
        let v = dot(cross(-lab, p01), la - p0) / denominator;
        Some(Vec3::new(t, u, v))
    } else {
        None
    }
}

#[test]
fn test_cross_product() {
    assert_eq!(
        cross(Vec3::new(1, 2, 3), Vec3::new(-2, 4, 6)),
        Vec3::new(0, -12, 8)
    );
}

#[test]
fn test_refract() {
    for _ in 0..100 {
        let a = random_unit_vector();
        let b = (random_unit_vector() - (2.0 * a)).unit_vector();
        let c = refract(&a, &b, 1.0);
        assert!((a - c).near_zero());
    }
}

#[test]
fn test_line_plane() {
    let tests = [[
        Vec3::new(1, 2, 3),
        Vec3::new(-1, 2, 1),
        Vec3::new(0, 2, -2),
        Vec3::new(5, 1, -1),
        Vec3::new(0, 9, 3),
    ]];
    for a in tests.iter() {
        if let Some(result) = line_plane_collision(a[0], a[1], a[2], a[3], a[4]) {
            assert_eq!(
                a[0] + a[1] * result[0],
                a[2] + a[3] * result[1] + a[4] * result[2]
            );
        } else {
            assert_eq!(dot(a[1], cross(a[3], a[4])), 0.0);
        }
    }
}
