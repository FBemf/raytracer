use anyhow::Result;
use image::{self, ImageBuffer, Rgb};

use std::sync::Arc;

use crate::hitting::Colour;
use crate::math::{clamp, Point3};

pub trait Texture: Send + Sync {
    fn value(&self, u: f64, v: f64, p: Point3) -> Colour;
    fn _print(&self) -> String;
}

pub struct SolidColour {
    pub colour: Colour,
}

impl Texture for SolidColour {
    fn value(&self, _u: f64, _v: f64, _p: Point3) -> Colour {
        self.colour
    }
    fn _print(&self) -> String {
        format!("Solid colour: {}", self.colour)
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
    fn _print(&self) -> String {
        format!(
            "Checkered: tile size {}, tiles ({}, {})",
            self.tile_size,
            self.odd._print(),
            self.even._print()
        )
    }
}

pub struct ImageTexture {
    pub image: ImageBuffer<Rgb<u8>, Vec<u8>>,
}

impl ImageTexture {
    pub fn from_file(filename: &str) -> Result<Arc<dyn Texture>> {
        let dyn_image = image::io::Reader::open(filename)?.decode()?;
        let image = dyn_image.into_rgb8();
        Ok(Arc::new(ImageTexture { image }))
    }
}

impl Texture for ImageTexture {
    fn value(&self, u: f64, v: f64, _p: Point3) -> Colour {
        let u = clamp(u, 0.0, 1.0);
        let v = 1.0 - clamp(v, 0.0, 1.0); // flip v

        let i = (u * self.image.width() as f64) as u32;
        let j = (v * self.image.height() as f64) as u32;
        let i = u32::min(i, self.image.width() - 1);
        let j = u32::min(j, self.image.height() - 1);

        let colour_scale = 1.0 / 255.0;
        let pixel = self.image.get_pixel(i, j);
        colour_scale * Colour::new(pixel[0], pixel[1], pixel[2])
    }
    fn _print(&self) -> String {
        format!("image texture")
    }
}
