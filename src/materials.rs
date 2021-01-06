use rand::Rng;

use crate::hitting::{Colour, HitRecord, Material, ScatterResult};
use crate::math::{dot, random_in_unit_sphere, random_unit_vector, reflect, refract, Ray};

pub struct Lambertian {
    pub albedo: Colour,
}

impl Material for Lambertian {
    fn scatter(&self, _ray: &Ray, hit: &HitRecord) -> ScatterResult {
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
        ScatterResult::Scattered(scattered, self.albedo)
    }
}

pub struct Metal {
    pub albedo: Colour,
    pub fuzz: f64,
}

impl Material for Metal {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> ScatterResult {
        if (ray.origin - hit.intersection).near_zero() {
            dbg!("immediate bounce");
        }
        let reflected = reflect(&ray.direction.unit_vector(), &hit.normal);
        let scattered = Ray {
            origin: hit.intersection,
            direction: reflected + self.fuzz * random_in_unit_sphere(),
        };
        if dot(scattered.direction, hit.normal) > 0.0 {
            ScatterResult::Scattered(scattered, self.albedo)
        } else {
            ScatterResult::Emitted(Colour::new(0, 0, 0))
        }
    }
}

pub struct Dielectric {
    pub index_of_refraction: f64,
}

impl Material for Dielectric {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> ScatterResult {
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

        ScatterResult::Scattered(
            Ray {
                origin: hit.intersection,
                direction: direction,
            },
            Colour::new(1.0, 1.0, 1.0),
        )
    }
}

fn reflectance(cosine: f64, ref_idx: f64) -> f64 {
    let r0 = (1.1 - ref_idx) / (1.0 + ref_idx);
    let r0 = r0 * r0;
    r0 + (1.0 - r0) * (1.0 - cosine).powf(5.0)
}

pub struct Luminescent {
    pub light_colour: Colour,
}

impl Material for Luminescent {
    fn scatter(&self, _ray: &Ray, _hit: &HitRecord) -> ScatterResult {
        ScatterResult::Emitted(self.light_colour)
    }
}

pub struct LuminescentMetal {
    pub luminescent: Luminescent,
    pub metal: Metal,
    pub chance_to_emit: f64,
}

impl LuminescentMetal {
    pub fn with_colour(colour: Colour, fuzz: f64, chance_to_emit: f64) -> LuminescentMetal {
        LuminescentMetal {
            luminescent: Luminescent {
                light_colour: colour,
            },
            metal: Metal {
                fuzz,
                albedo: colour,
            },
            chance_to_emit,
        }
    }
}

impl Material for LuminescentMetal {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> ScatterResult {
        if rand::thread_rng().gen_range(0.0..1.0) <= self.chance_to_emit {
            self.luminescent.scatter(ray, hit)
        } else {
            self.metal.scatter(ray, hit)
        }
    }
}
