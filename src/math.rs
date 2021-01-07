use rand::Rng;
use std::fmt;

pub type Point3 = Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn new<T: Into<f64>>(x: T, y: T, z: T) -> Vec3 {
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
    fn add(self, other: Self) -> Self {
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
    fn sub(self, other: Self) -> Self {
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
    type Output = Vec3;
    fn mul(self, other: f64) -> Vec3 {
        Vec3 {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
        }
    }
}

impl std::ops::Mul<Vec3> for f64 {
    type Output = Vec3;
    fn mul(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: other.x * self,
            y: other.y * self,
            z: other.z * self,
        }
    }
}

impl std::ops::MulAssign<f64> for Vec3 {
    fn mul_assign(&mut self, other: f64) {
        self.x *= other;
        self.y *= other;
        self.z *= other;
    }
}

impl std::ops::Div<f64> for Vec3 {
    type Output = Vec3;
    fn div(self, other: f64) -> Vec3 {
        Vec3 {
            x: self.x / other,
            y: self.y / other,
            z: self.z / other,
        }
    }
}

impl std::ops::DivAssign<f64> for Vec3 {
    fn div_assign(&mut self, other: f64) {
        self.x /= other;
        self.y /= other;
        self.z /= other;
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

impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

#[derive(Debug)]
pub struct Ray {
    pub origin: Point3,
    pub direction: Vec3,
    pub time: f64,
}

impl Ray {
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
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
        );
        if p.length_squared() <= 1.0 {
            return p;
        }
    }
}

pub fn random_in_unit_disc() -> Vec3 {
    let mut rng = rand::thread_rng();
    loop {
        let p = Vec3::new(rng.gen_range(0.0..1.0), rng.gen_range(0.0..1.0), 0.0);
        if p.length_squared() <= 1.0 {
            return p;
        }
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
