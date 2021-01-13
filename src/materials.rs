use rand::Rng;

use std::f64::consts::PI;
use std::sync::Arc;

use crate::hitting::{Colour, HitRecord, Material};
use crate::math::{dot, random_in_unit_sphere, random_unit_vector, reflect, refract, Ray};
use crate::textures::{SolidColour, Texture};

pub struct Lambertian {
    pub albedo: Arc<dyn Texture>,
}

impl Lambertian {
    pub fn with_colour(colour: Colour) -> Arc<dyn Material> {
        Arc::new(Lambertian {
            albedo: Arc::new(SolidColour { colour }),
        })
    }
    pub fn with_texture(texture: &Arc<dyn Texture>) -> Arc<dyn Material> {
        Arc::new(Lambertian {
            albedo: Arc::clone(texture),
        })
    }
}

impl Material for Lambertian {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> Option<(Ray, Colour)> {
        let scatter_direction = hit.normal + random_unit_vector();
        // catch degenerate scatter direction
        let scatter_direction = if scatter_direction.near_zero() {
            hit.normal
        } else {
            scatter_direction
        };
        let scattered = Ray::new(hit.intersection, scatter_direction, ray.time);
        Some((
            scattered,
            self.albedo
                .value(hit.surface_u, hit.surface_v, hit.intersection),
        ))
    }
    fn _print(&self) -> String {
        format!("Lambertian: {}", self.albedo._print())
    }
}

pub struct Metal {
    pub albedo: Colour,
    pub fuzz: f64,
}

impl Material for Metal {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> Option<(Ray, Colour)> {
        let reflected = reflect(&ray.direction.unit_vector(), &hit.normal);
        let scattered = Ray::new(
            hit.intersection,
            reflected + self.fuzz * random_in_unit_sphere(),
            ray.time,
        );
        if dot(scattered.direction, hit.normal) > 0.0 {
            Some((scattered, self.albedo))
        } else {
            None
        }
    }
    fn _print(&self) -> String {
        format!("Metal: albedo {}, fuzz {}", self.albedo, self.fuzz)
    }
}

pub struct Dielectric {
    pub index_of_refraction: f64,
}

impl Material for Dielectric {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> Option<(Ray, Colour)> {
        let refraction_ratio = if hit.front_face {
            1.0 / self.index_of_refraction
        } else {
            self.index_of_refraction
        };
        let unit_direction = ray.direction.unit_vector();

        let cos_theta = f64::min(dot(-unit_direction, hit.normal), 1.0);
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        let cannot_refract = refraction_ratio * sin_theta > 1.0;
        let random_fraction = rand::thread_rng().gen_range(0.0..1.0);
        let direction =
            if cannot_refract || reflectance(cos_theta, refraction_ratio) > random_fraction {
                // cannot refract
                reflect(&unit_direction, &hit.normal)
            } else {
                refract(&unit_direction, &hit.normal, refraction_ratio)
            };

        Some((
            Ray::new(hit.intersection, direction, ray.time),
            Colour::new(1.0, 1.0, 1.0),
        ))
    }
    fn _print(&self) -> String {
        format!("Dielectric: ior {}", self.index_of_refraction)
    }
}

fn reflectance(cosine: f64, ref_idx: f64) -> f64 {
    let r0 = (1.1 - ref_idx) / (1.0 + ref_idx);
    let r0 = r0 * r0;
    r0 + (1.0 - r0) * (1.0 - cosine).powf(5.0)
}

pub struct DiffuseLight {
    pub emit: Arc<dyn Texture>,
}

impl DiffuseLight {
    pub fn with_colour(colour: Colour) -> Arc<dyn Material> {
        Arc::new(DiffuseLight {
            emit: Arc::new(SolidColour { colour }),
        })
    }
}

impl Material for DiffuseLight {
    fn scatter(&self, _ray: &Ray, _hit: &HitRecord) -> Option<(Ray, Colour)> {
        None
    }
    fn emitted(&self, hit: &HitRecord) -> Colour {
        if hit.front_face {
            self.emit
                .value(hit.surface_u, hit.surface_v, hit.intersection)
        } else {
            Colour::new(0, 0, 0)
        }
    }
    fn _print(&self) -> String {
        format!("Diffuse light: {}", self.emit._print())
    }
}

pub struct Isotropic {
    pub albedo: Arc<dyn Texture>,
}

impl Material for Isotropic {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> Option<(Ray, Colour)> {
        Some((
            Ray::new(hit.intersection, random_in_unit_sphere(), ray.time),
            self.albedo
                .value(hit.surface_u, hit.surface_v, hit.intersection),
        ))
    }
    fn _print(&self) -> String {
        format!("Isotropic: {}", self.albedo._print())
    }
}

pub struct Checkered {
    pub odd: Arc<dyn Material>,
    pub even: Arc<dyn Material>,
    pub tile_density: f64,
}

impl Material for Checkered {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> Option<(Ray, Colour)> {
        let sines = (self.tile_density * PI * hit.surface_u).sin()
            * (self.tile_density * PI * hit.surface_v).sin();
        if sines < 0.0 {
            self.odd.scatter(ray, hit)
        } else {
            self.even.scatter(ray, hit)
        }
    }
    fn emitted(&self, hit: &HitRecord) -> Colour {
        let sines =
            (self.tile_density * hit.surface_u).sin() * (self.tile_density * hit.surface_v).sin();
        if sines < 0.0 {
            self.odd.emitted(hit)
        } else {
            self.even.emitted(hit)
        }
    }
    fn _print(&self) -> String {
        format!(
            "Checkered: tile size {}, tiles ({}, {})",
            self.tile_density,
            self.odd._print(),
            self.even._print()
        )
    }
}
