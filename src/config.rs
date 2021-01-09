use anyhow::{anyhow, bail, Result};
use json5;
use serde_derive::Deserialize;

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use crate::hitting::{BVHNode, Colour, Hittable, Material};
use crate::materials;
use crate::math::{Point3, Vec3};
use crate::objects;
use crate::textures::{self, Texture};
use crate::transforms;

fn load_config(filename: &str) -> Result<()> {
    let mut config_string = String::new();
    File::open(filename)?.read_to_string(&mut config_string)?;
    let config = json5::from_str(&config_string)?;
    let textures = build_textures(&config)?;
    let materials = build_materials(&config, &textures)?;
    let hittables = build_hittables(&config, &materials)?;
    let world = config
        .world
        .iter()
        .map(|s| {
            let a = hittables.get(&s as &str).and_then(|&a| Some(Box::new(*a)));
            //.ok_or(anyhow!("Object {} does not exist", s))
        })
        .collect::<Result<Vec<Box<dyn Hittable>>>>()?;
    let world = BVHNode::from_vec(world, 0.0, 1.0);
    unimplemented!();
}

fn build_textures(master_config: &MasterConfig) -> Result<HashMap<&str, Arc<dyn Texture>>> {
    let mut texture_list: HashMap<&str, Arc<dyn Texture>> = HashMap::new();
    let mut texture_configs: VecDeque<(&str, &TextureConfig)> = master_config
        .textures
        .iter()
        .map(|(s, t)| (s as &str, t))
        .collect();
    while texture_configs.len() != 0 {
        for _ in 0..texture_configs.len() {
            let (name, texture) = texture_configs.pop_front().unwrap();
            match texture {
                TextureConfig::SolidColour(config) => {
                    texture_list.insert(
                        name,
                        Arc::new(textures::SolidColour {
                            colour: Colour::new(
                                config.colour[0],
                                config.colour[1],
                                config.colour[2],
                            ),
                        }),
                    );
                    break;
                }
                TextureConfig::Checkered(config) => {
                    if texture_list.contains_key(&config.odd as &str)
                        && texture_list.contains_key(&config.even as &str)
                    {
                        texture_list.insert(
                            name,
                            Arc::new(textures::Checkered {
                                odd: Arc::clone(texture_list.get(&config.odd as &str).unwrap()),
                                even: Arc::clone(texture_list.get(&config.even as &str).unwrap()),
                                tile_size: config.tile_size,
                            }),
                        );
                    } else {
                        texture_configs.push_back((name, texture));
                    }
                }
                TextureConfig::ImageTexture(config) => {
                    texture_list.insert(name, textures::ImageTexture::from_file(&config.filename)?);
                }
            }
        }
        bail!(
            "Texture {} is impossible to construct",
            texture_configs[0].0
        );
    }
    Ok(texture_list)
}

fn build_materials<'a>(
    master_config: &'a MasterConfig,
    textures: &HashMap<&str, Arc<dyn Texture>>,
) -> Result<HashMap<&'a str, Arc<dyn Material>>> {
    let mut material_list: HashMap<&str, Arc<dyn Material>> = HashMap::new();
    let mut material_configs: VecDeque<(&str, &MaterialConfig)> = master_config
        .materials
        .iter()
        .map(|(s, t)| (s as &str, t))
        .collect();
    while material_configs.len() != 0 {
        for _ in 0..material_configs.len() {
            let (name, material) = material_configs.pop_front().unwrap();
            match material {
                MaterialConfig::Lambertian(config) => {
                    let texture = textures
                        .get(&config.texture as &str)
                        .ok_or(anyhow!("Texture {} does not exist", config.texture))?;
                    material_list.insert(name, materials::Lambertian::with_texture(texture));
                }
                MaterialConfig::Metal(config) => {
                    material_list.insert(
                        name,
                        Arc::new(materials::Metal {
                            albedo: Colour::new(
                                config.albedo[0],
                                config.albedo[1],
                                config.albedo[2],
                            ),
                            fuzz: config.fuzz,
                        }),
                    );
                }
                MaterialConfig::Dielectric(config) => {
                    material_list.insert(
                        name,
                        Arc::new(materials::Dielectric {
                            index_of_refraction: config.index_of_refraction,
                        }),
                    );
                }
                MaterialConfig::DiffuseLight(config) => {
                    let texture = textures
                        .get(&config.emit as &str)
                        .ok_or(anyhow!("Texture {} does not exist", config.emit))?;
                    material_list.insert(
                        name,
                        Arc::new(materials::DiffuseLight {
                            emit: Arc::clone(texture),
                        }),
                    );
                }
                MaterialConfig::Isotropic(config) => {
                    let texture = textures
                        .get(&config.albedo as &str)
                        .ok_or(anyhow!("Texture {} does not exist", config.albedo))?;
                    material_list.insert(
                        name,
                        Arc::new(materials::DiffuseLight {
                            emit: Arc::clone(texture),
                        }),
                    );
                }
            }
        }
        bail!(
            "Material {} is impossible to construct",
            material_configs[0].0
        );
    }
    Ok(material_list)
}

fn build_hittables<'a>(
    master_config: &'a MasterConfig,
    materials: &HashMap<&str, Arc<dyn Material>>,
) -> Result<HashMap<&'a str, Arc<dyn Hittable>>> {
    let mut hittable_list: HashMap<&str, Arc<dyn Hittable>> = HashMap::new();
    let mut hittable_configs: VecDeque<(&str, &ObjectConfig)> = master_config
        .objects
        .iter()
        .map(|(s, t)| (s as &str, t))
        .collect();
    while hittable_configs.len() != 0 {
        for _ in 0..hittable_configs.len() {
            let (name, hittable) = hittable_configs.pop_front().unwrap();
            match hittable {
                ObjectConfig::Sphere(config) => {
                    let material = materials
                        .get(&config.material as &str)
                        .ok_or(anyhow!("Material {} does not exist", config.material))?;
                    hittable_list.insert(
                        name,
                        objects::Sphere::new(
                            Point3::new(config.centre[0], config.centre[1], config.centre[2]),
                            config.radius,
                            material,
                        )
                        .into(),
                    );
                }
                ObjectConfig::MovingSphere(config) => {
                    let material = materials
                        .get(&config.material as &str)
                        .ok_or(anyhow!("Material {} does not exist", config.material))?;
                    hittable_list.insert(
                        name,
                        objects::MovingSphere::new(
                            Point3::new(config.centre0[0], config.centre0[1], config.centre0[2]),
                            Point3::new(config.centre1[0], config.centre1[1], config.centre1[2]),
                            config.time0,
                            config.time1,
                            config.radius,
                            material,
                        )
                        .into(),
                    );
                }
                ObjectConfig::Block(config) => {
                    let material = materials
                        .get(&config.material as &str)
                        .ok_or(anyhow!("Material {} does not exist", config.material))?;
                    hittable_list.insert(
                        name,
                        objects::Block::new(
                            Point3::new(config.corner0[0], config.corner0[1], config.corner0[2]),
                            Point3::new(config.corner1[0], config.corner1[1], config.corner1[2]),
                            material,
                        )
                        .into(),
                    );
                }
                ObjectConfig::Rect(config) => {
                    let material = materials
                        .get(&config.material as &str)
                        .ok_or(anyhow!("Material {} does not exist", config.material))?;
                    hittable_list.insert(
                        name,
                        if config.corner0[0] == config.corner1[0] {
                            objects::YZRect::new(
                                config.corner0[1],
                                config.corner0[2],
                                config.corner1[1],
                                config.corner1[2],
                                config.corner0[0],
                                material,
                                config.facing_forward,
                            )
                            .into()
                        } else if config.corner0[1] == config.corner1[1] {
                            objects::YZRect::new(
                                config.corner0[0],
                                config.corner0[2],
                                config.corner1[0],
                                config.corner1[2],
                                config.corner0[1],
                                material,
                                config.facing_forward,
                            )
                            .into()
                        } else if config.corner0[2] == config.corner1[2] {
                            objects::XYRect::new(
                                config.corner0[0],
                                config.corner0[1],
                                config.corner1[0],
                                config.corner1[1],
                                config.corner0[2],
                                material,
                                config.facing_forward,
                            )
                            .into()
                        } else {
                            bail!("Rectangles are 2d; corner0 and corner1 must be equal along one axis")
                        },
                    );
                }
                ObjectConfig::ConstantMedium(config) => {
                    if hittable_list.contains_key(&config.boundary as &str) {
                        let material = materials
                            .get(&config.phase_function as &str)
                            .ok_or(anyhow!("Material {} does not exist", config.phase_function))?;
                        let boundary = hittable_list.get(&config.boundary as &str).unwrap();
                        let object =
                            objects::ConstantMedium::new(boundary, material, config.density);
                        hittable_list.insert(name, object.into());
                    } else {
                        hittable_configs.push_back((name, hittable));
                    }
                }
                ObjectConfig::Translate(config) => {
                    if hittable_list.contains_key(&config.prototype as &str) {
                        let prototype = hittable_list.get(&config.prototype as &str).unwrap();
                        let object = transforms::Translate::translate(
                            prototype,
                            Vec3::new(config.offset[0], config.offset[1], config.offset[2]),
                        );
                        hittable_list.insert(name, object.into());
                    } else {
                        hittable_configs.push_back((name, hittable));
                    }
                }
                ObjectConfig::RotateX(config) => {
                    if hittable_list.contains_key(&config.prototype as &str) {
                        let prototype = hittable_list.get(&config.prototype as &str).unwrap();
                        let object = transforms::RotateX::by_degrees(prototype, config.degrees);
                        hittable_list.insert(name, object.into());
                    } else {
                        hittable_configs.push_back((name, hittable));
                    }
                }
                ObjectConfig::RotateY(config) => {
                    if hittable_list.contains_key(&config.prototype as &str) {
                        let prototype = hittable_list.get(&config.prototype as &str).unwrap();
                        let object = transforms::RotateY::by_degrees(prototype, config.degrees);
                        hittable_list.insert(name, object.into());
                    } else {
                        hittable_configs.push_back((name, hittable));
                    }
                }
                ObjectConfig::RotateZ(config) => {
                    if hittable_list.contains_key(&config.prototype as &str) {
                        let prototype = hittable_list.get(&config.prototype as &str).unwrap();
                        let object = transforms::RotateZ::by_degrees(prototype, config.degrees);
                        hittable_list.insert(name, object.into());
                    } else {
                        hittable_configs.push_back((name, hittable));
                    }
                }
            }
        }
        bail!(
            "Object {} is impossible to construct",
            hittable_configs[0].0
        );
    }
    Ok(hittable_list)
}

#[derive(Deserialize)]
struct MasterConfig {
    textures: HashMap<String, TextureConfig>,
    materials: HashMap<String, MaterialConfig>,
    objects: HashMap<String, ObjectConfig>,
    world: Vec<String>,
}

#[derive(Deserialize)]
enum TextureConfig {
    SolidColour(SolidColourConfig),
    Checkered(CheckeredConfig),
    ImageTexture(ImageTextureConfig),
}

#[derive(Deserialize)]
struct SolidColourConfig {
    colour: [f64; 3],
}

#[derive(Deserialize)]
struct CheckeredConfig {
    odd: String,
    even: String,
    tile_size: f64,
}

#[derive(Deserialize)]
struct ImageTextureConfig {
    filename: String,
}

#[derive(Deserialize)]
enum MaterialConfig {
    Lambertian(LambertianConfig),
    Metal(MetalConfig),
    Dielectric(DielectricConfig),
    DiffuseLight(DiffuseLightConfig),
    Isotropic(IsotropicConfig),
}

#[derive(Deserialize)]
struct LambertianConfig {
    texture: String,
}

#[derive(Deserialize)]
struct MetalConfig {
    fuzz: f64,
    albedo: [f64; 3],
}

#[derive(Deserialize)]
struct DielectricConfig {
    index_of_refraction: f64,
}

#[derive(Deserialize)]
struct DiffuseLightConfig {
    emit: String,
}

#[derive(Deserialize)]
struct IsotropicConfig {
    albedo: String,
}

#[derive(Deserialize)]
enum ObjectConfig {
    Sphere(SphereConfig),
    MovingSphere(MovingSphereConfig),
    Block(BlockConfig),
    Rect(RectConfig),
    ConstantMedium(ConstantMediumConfig),
    Translate(TranslateConfig),
    RotateX(RotateXConfig),
    RotateY(RotateYConfig),
    RotateZ(RotateZConfig),
}

#[derive(Deserialize)]
struct SphereConfig {
    centre: [f64; 3],
    radius: f64,
    material: String,
}

#[derive(Deserialize)]
struct MovingSphereConfig {
    centre0: [f64; 3],
    centre1: [f64; 3],
    time0: f64,
    time1: f64,
    radius: f64,
    material: String,
}

#[derive(Deserialize)]
struct BlockConfig {
    corner0: [f64; 3],
    corner1: [f64; 3],
    material: String,
}

#[derive(Deserialize)]
struct RectConfig {
    // these must share one coordinate or it'll error out
    corner0: [f64; 3],
    corner1: [f64; 3],
    facing_forward: bool,
    material: String,
}

#[derive(Deserialize)]
struct ConstantMediumConfig {
    boundary: String,
    phase_function: String,
    density: f64,
}

#[derive(Deserialize)]
struct TranslateConfig {
    prototype: String,
    offset: [f64; 3],
}

#[derive(Deserialize)]
struct RotateXConfig {
    prototype: String,
    degrees: f64,
}

#[derive(Deserialize)]
struct RotateYConfig {
    prototype: String,
    degrees: f64,
}

#[derive(Deserialize)]
struct RotateZConfig {
    prototype: String,
    degrees: f64,
}
