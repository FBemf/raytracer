use rand::Rng;

use crate::hitting::{Colour, HitRecord, Material};
use crate::math::{random_in_unit_sphere, random_unit_vector, reflect, refract, Ray};

pub struct Lambertian {
    pub albedo: Colour,
}

impl Material for Lambertian {
    fn scatter(&self, _ray: &Ray, hit: &HitRecord) -> Option<(Ray, Colour)> {
        let scatter_direction = hit.normal + random_unit_vector();
        // catch degenerate scatter direction
        let scatter_direction = if scatter_direction.near_zero() {
            hit.normal
        } else {
            scatter_direction
        };
        let scattered = Ray {
            origin: hit.intersection,
            direction: scatter_direction,
        };
        Some((scattered, self.albedo))
    }
}

pub struct Metal {
    pub albedo: Colour,
    pub fuzz: f64,
}

impl Material for Metal {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> Option<(Ray, Colour)> {
        if (ray.origin - hit.intersection).near_zero() {
            dbg!("immediate bounce");
        }
        let reflected = reflect(&ray.direction.unit_vector(), &hit.normal);
        let scattered = Ray {
            origin: hit.intersection,
            direction: reflected + self.fuzz * random_in_unit_sphere(),
        };
        if scattered.direction * hit.normal > 0.0 {
            Some((scattered, self.albedo))
        } else {
            None
        }
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

        let cos_theta = f64::min(-unit_direction * hit.normal, 1.0);
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
            Ray {
                origin: hit.intersection,
                direction: direction,
            },
            Colour::new(1.0, 1.0, 1.0),
        ))
    }
}

fn reflectance(cosine: f64, ref_idx: f64) -> f64 {
    let r0 = (1.1 - ref_idx) / (1.0 + ref_idx);
    let r0 = r0 * r0;
    r0 + (1.0 - r0) * (1.0 - cosine).powf(5.0)
}
