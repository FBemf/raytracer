use std::sync::Arc;

use crate::hitting::Colour;
use crate::math::Point3;

pub trait Texture: Send + Sync {
    fn value(&self, u: f64, v: f64, p: Point3) -> Colour;
}

pub struct SolidColour {
    pub colour: Colour,
}

impl Texture for SolidColour {
    fn value(&self, _u: f64, _v: f64, _p: Point3) -> Colour {
        self.colour
    }
}

pub struct Checkered {
    pub odd: Arc<dyn Texture>,
    pub even: Arc<dyn Texture>,
    pub tile_size: f64,
}

impl Texture for Checkered {
    fn value(&self, u: f64, v: f64, p: Point3) -> Colour {
        let sines = (self.tile_size * p.x).sin()
            * (self.tile_size * p.y).sin()
            * (self.tile_size * p.z).sin();
        if sines < 0.0 {
            self.odd.value(u, v, p)
        } else {
            self.even.value(u, v, p)
        }
    }
}
