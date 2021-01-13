use anyhow::{anyhow, bail, Result};
use json5;
use serde_derive::Deserialize;

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use crate::camera::{gradient_background, Camera, Sky};
use crate::hitting::{BVHNode, Colour, Hittable, Material};
use crate::materials;
use crate::math::{Point3, Vec3};
use crate::objects;
use crate::textures::{self, Texture};
use crate::transforms;

pub fn load_config(filename: &PathBuf) -> Result<(Camera, Arc<dyn Hittable>, Sky, f64)> {
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
            hittables
                .get(&s as &str)
                .and_then(|a| Some(Arc::clone(a)))
                .ok_or(anyhow!("Object {} does not exist", s))
        })
        .collect::<Result<Vec<Arc<dyn Hittable>>>>()?;
    let world = BVHNode::from_vec(world, config.camera.start_time, config.camera.end_time);
    let aspect_ratio = config.camera.aspect_ratio[0] / config.camera.aspect_ratio[1];
    let camera = Camera::new(
        Point3::new(
            config.camera.look_from[0],
            config.camera.look_from[1],
            config.camera.look_from[2],
        ),
        Point3::new(
            config.camera.look_at[0],
            config.camera.look_at[1],
            config.camera.look_at[2],
        ),
        Point3::new(
            config.camera.direction_up[0],
            config.camera.direction_up[1],
            config.camera.direction_up[2],
        ),
        config.camera.vertical_fov,
        aspect_ratio,
        config.camera.aperture,
        config.camera.focus_dist,
        config.camera.start_time,
        config.camera.end_time,
    );
    let sky = get_background(config.background);
    Ok((camera, world, sky, aspect_ratio))
}

fn build_textures(master_config: &MasterConfig) -> Result<HashMap<&str, Arc<dyn Texture>>> {
    let mut texture_list: HashMap<&str, Arc<dyn Texture>> = HashMap::new();
    let mut texture_configs: VecDeque<(&str, &TextureConfig)> = master_config
        .textures
        .iter()
        .map(|(s, t)| (s as &str, t))
        .collect();
    'begin_search: while texture_configs.len() != 0 {
        for _ in 0..texture_configs.len() {
            let (name, texture) = texture_configs.pop_front().unwrap();
            match texture {
                TextureConfig::SolidColour { colour } => {
                    texture_list.insert(
                        name,
                        Arc::new(textures::SolidColour {
                            colour: Colour::new(colour[0], colour[1], colour[2]),
                        }),
                    );
                    continue 'begin_search;
                }
                TextureConfig::ImageTexture { filename } => {
                    texture_list.insert(name, textures::ImageTexture::from_file(&filename)?);
                    continue 'begin_search;
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
    'begin_search: while material_configs.len() != 0 {
        for _ in 0..material_configs.len() {
            let (name, material) = material_configs.pop_front().unwrap();
            match material {
                MaterialConfig::Lambertian { texture } => {
                    let texture = textures
                        .get(&texture as &str)
                        .ok_or(anyhow!("Texture {} does not exist", texture))?;
                    material_list.insert(name, materials::Lambertian::with_texture(texture));
                    continue 'begin_search;
                }
                MaterialConfig::Metal { fuzz, albedo } => {
                    material_list.insert(
                        name,
                        Arc::new(materials::Metal {
                            albedo: Colour::new(albedo[0], albedo[1], albedo[2]),
                            fuzz: *fuzz,
                        }),
                    );
                    continue 'begin_search;
                }
                MaterialConfig::Dielectric {
                    index_of_refraction,
                } => {
                    material_list.insert(
                        name,
                        Arc::new(materials::Dielectric {
                            index_of_refraction: *index_of_refraction,
                        }),
                    );
                    continue 'begin_search;
                }
                MaterialConfig::DiffuseLight { emit } => {
                    let texture = textures
                        .get(&emit as &str)
                        .ok_or(anyhow!("Texture {} does not exist", emit))?;
                    material_list.insert(
                        name,
                        Arc::new(materials::DiffuseLight {
                            emit: Arc::clone(texture),
                        }),
                    );
                    continue 'begin_search;
                }
                MaterialConfig::Isotropic { albedo } => {
                    let texture = textures
                        .get(&albedo as &str)
                        .ok_or(anyhow!("Texture {} does not exist", albedo))?;
                    material_list.insert(
                        name,
                        Arc::new(materials::Isotropic {
                            albedo: Arc::clone(texture),
                        }),
                    );
                    continue 'begin_search;
                }
                MaterialConfig::Checkered {
                    odd,
                    even,
                    tile_density,
                } => {
                    if material_list.contains_key(&odd as &str)
                        && material_list.contains_key(&even as &str)
                    {
                        material_list.insert(
                            name,
                            Arc::new(materials::Checkered {
                                odd: Arc::clone(&material_list[odd as &str]),
                                even: Arc::clone(&material_list[even as &str]),
                                tile_density: *tile_density,
                            }),
                        );
                        continue 'begin_search;
                    }
                    // couldn't build yet
                    material_configs.push_back((name, material));
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
    'begin_search: while hittable_configs.len() != 0 {
        for _ in 0..hittable_configs.len() {
            let (name, hittable) = hittable_configs.pop_front().unwrap();
            match hittable {
                ObjectConfig::Sphere {
                    centre,
                    radius,
                    material,
                } => {
                    let material = materials
                        .get(&material as &str)
                        .ok_or(anyhow!("Material {} does not exist", material))?;
                    hittable_list.insert(
                        name,
                        objects::Sphere::new(
                            Point3::new(centre[0], centre[1], centre[2]),
                            *radius,
                            material,
                        ),
                    );
                    continue 'begin_search;
                }
                ObjectConfig::MovingSphere {
                    centre0,
                    centre1,
                    time0,
                    time1,
                    radius,
                    material,
                } => {
                    let material = materials
                        .get(&material as &str)
                        .ok_or(anyhow!("Material {} does not exist", material))?;
                    hittable_list.insert(
                        name,
                        objects::MovingSphere::new(
                            Point3::new(centre0[0], centre0[1], centre0[2]),
                            Point3::new(centre1[0], centre1[1], centre1[2]),
                            *time0,
                            *time1,
                            *radius,
                            material,
                        ),
                    );
                    continue 'begin_search;
                }
                ObjectConfig::Block {
                    corner0,
                    corner1,
                    material,
                } => {
                    let material = materials
                        .get(&material as &str)
                        .ok_or(anyhow!("Material {} does not exist", material))?;
                    hittable_list.insert(
                        name,
                        objects::Block::new(
                            Point3::new(corner0[0], corner0[1], corner0[2]),
                            Point3::new(corner1[0], corner1[1], corner1[2]),
                            material,
                        ),
                    );
                    continue 'begin_search;
                }
                ObjectConfig::Rect {
                    corner0,
                    corner1,
                    facing_forward,
                    material,
                } => {
                    let material = materials
                        .get(&material as &str)
                        .ok_or(anyhow!("Material {} does not exist", material))?;
                    hittable_list.insert(
                        name,
                        if corner0[0] == corner1[0] {
                            objects::YZRect::new(
                                corner0[1],
                                corner1[1],
                                corner0[2],
                                corner1[2],
                                corner0[0],
                                material,
                                *facing_forward,
                            )
                        } else if corner0[1] == corner1[1] {
                            objects::XZRect::new(
                                corner0[0],
                                corner1[0],
                                corner0[2],
                                corner1[2],
                                corner0[1],
                                material,
                                *facing_forward,
                            )
                        } else if corner0[2] == corner1[2] {
                            objects::XYRect::new(
                                corner0[0],
                                corner1[0],
                                corner0[1],
                                corner1[1],
                                corner0[2],
                                material,
                                *facing_forward,
                            )
                        } else {
                            bail!("Rectangles are 2d; corner0 and corner1 must be equal along one axis")
                        },
                    );
                    continue 'begin_search;
                }
                ObjectConfig::Triangle {
                    point0,
                    point1,
                    point2,
                    material,
                } => {
                    let material = materials
                        .get(&material as &str)
                        .ok_or(anyhow!("Material {} does not exist", material))?;
                    hittable_list.insert(
                        name,
                        objects::Triangle::new(
                            Point3::new(point0[0], point0[1], point0[2]),
                            Point3::new(point1[0], point1[1], point1[2]),
                            Point3::new(point2[0], point2[1], point2[2]),
                            material,
                        ),
                    );
                    continue 'begin_search;
                }
                ObjectConfig::Plane {
                    point0,
                    point1,
                    point2,
                    uv_repeat,
                    material,
                } => {
                    let material = materials
                        .get(&material as &str)
                        .ok_or(anyhow!("Material {} does not exist", material))?;
                    hittable_list.insert(
                        name,
                        objects::Plane::new(
                            Point3::new(point0[0], point0[1], point0[2]),
                            Point3::new(point1[0], point1[1], point1[2]),
                            Point3::new(point2[0], point2[1], point2[2]),
                            *uv_repeat,
                            material,
                        ),
                    );
                    continue 'begin_search;
                }
                ObjectConfig::Mesh {
                    filename,
                    object_name,
                    material,
                } => {
                    let material = materials
                        .get(&material as &str)
                        .ok_or(anyhow!("Material {} does not exist", material))?;
                    let object_name = if let Some(n) = object_name { n } else { "" };
                    let triangles = objects::load_mesh(filename, object_name, material)?;
                    hittable_list.insert(name, triangles);
                    continue 'begin_search;
                }
                ObjectConfig::Spotlight {
                    look_from,
                    look_at,
                    length,
                    width,
                    light,
                } => {
                    hittable_list.insert(
                        name,
                        objects::Spotlight::new(
                            Point3::new(look_from[0], look_from[1], look_from[2]),
                            Point3::new(look_at[0], look_at[1], look_at[2]),
                            *length,
                            *width,
                            Colour::new(light[0], light[1], light[2]),
                        ),
                    );
                    continue 'begin_search;
                }
                ObjectConfig::ConstantMedium {
                    boundary,
                    phase_function,
                    density,
                } => {
                    if hittable_list.contains_key(&boundary as &str) {
                        let material = materials
                            .get(&phase_function as &str)
                            .ok_or(anyhow!("Material {} does not exist", phase_function))?;
                        let boundary = hittable_list.get(&boundary as &str).unwrap();
                        let object = objects::ConstantMedium::new(boundary, material, *density);
                        hittable_list.insert(name, object);
                        continue 'begin_search;
                    } else {
                        hittable_configs.push_back((name, hittable));
                    }
                }
                ObjectConfig::Translate { prototype, offset } => {
                    if hittable_list.contains_key(&prototype as &str) {
                        let prototype = hittable_list.get(&prototype as &str).unwrap();
                        let object = transforms::Translate::translate(
                            prototype,
                            Vec3::new(offset[0], offset[1], offset[2]),
                        );
                        hittable_list.insert(name, object.into());
                        continue 'begin_search;
                    } else {
                        hittable_configs.push_back((name, hittable));
                    }
                }
                ObjectConfig::RotateX { prototype, degrees } => {
                    if hittable_list.contains_key(&prototype as &str) {
                        let prototype = hittable_list.get(&prototype as &str).unwrap();
                        let object = transforms::RotateX::by_degrees(prototype, *degrees);
                        hittable_list.insert(name, object.into());
                        continue 'begin_search;
                    } else {
                        hittable_configs.push_back((name, hittable));
                    }
                }
                ObjectConfig::RotateY { prototype, degrees } => {
                    if hittable_list.contains_key(&prototype as &str) {
                        let prototype = hittable_list.get(&prototype as &str).unwrap();
                        let object = transforms::RotateY::by_degrees(prototype, *degrees);
                        hittable_list.insert(name, object.into());
                        continue 'begin_search;
                    } else {
                        hittable_configs.push_back((name, hittable));
                    }
                }
                ObjectConfig::RotateZ { prototype, degrees } => {
                    if hittable_list.contains_key(&prototype as &str) {
                        let prototype = hittable_list.get(&prototype as &str).unwrap();
                        let object = transforms::RotateZ::by_degrees(prototype, *degrees);
                        hittable_list.insert(name, object.into());
                        continue 'begin_search;
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

fn get_background(config: BackgroundConfig) -> Sky {
    match config {
        BackgroundConfig::PlainColour { colour: [r, g, b] } => {
            Box::new(move |_| Colour::new(r, g, b))
        }
        BackgroundConfig::Gradient {
            direction,
            colour0,
            colour1,
        } => {
            let direction = Vec3::new(direction[0], direction[1], direction[2]);
            let colour0 = Colour::new(colour0[0], colour0[1], colour0[2]);
            let colour1 = Colour::new(colour1[0], colour1[1], colour1[2]);
            gradient_background(direction, colour0, colour1)
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct MasterConfig {
    camera: CameraConfig,
    background: BackgroundConfig,
    textures: HashMap<String, TextureConfig>,
    materials: HashMap<String, MaterialConfig>,
    objects: HashMap<String, ObjectConfig>,
    world: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "camelCase")]
enum BackgroundConfig {
    #[serde(rename_all = "camelCase")]
    PlainColour { colour: [f64; 3] },
    #[serde(rename_all = "camelCase")]
    Gradient {
        direction: [f64; 3],
        colour0: [f64; 3],
        colour1: [f64; 3],
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CameraConfig {
    look_from: [f64; 3],
    look_at: [f64; 3],
    direction_up: [f64; 3],
    #[serde(rename = "fieldOfView")]
    vertical_fov: f64,
    aspect_ratio: [f64; 2],
    aperture: f64,
    #[serde(rename = "distanceToFocus")]
    focus_dist: f64,
    start_time: f64,
    end_time: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "camelCase")]
enum TextureConfig {
    #[serde(rename_all = "camelCase")]
    SolidColour { colour: [f64; 3] },
    #[serde(rename_all = "camelCase")]
    ImageTexture { filename: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "camelCase")]
enum MaterialConfig {
    #[serde(rename_all = "camelCase")]
    Lambertian { texture: String },
    #[serde(rename_all = "camelCase")]
    Metal { fuzz: f64, albedo: [f64; 3] },
    #[serde(rename_all = "camelCase")]
    Dielectric { index_of_refraction: f64 },
    #[serde(rename_all = "camelCase")]
    DiffuseLight { emit: String },
    #[serde(rename_all = "camelCase")]
    Isotropic { albedo: String },
    #[serde(rename_all = "camelCase")]
    Checkered {
        odd: String,
        even: String,
        tile_density: f64,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "camelCase")]
enum ObjectConfig {
    #[serde(rename_all = "camelCase")]
    Sphere {
        centre: [f64; 3],
        radius: f64,
        material: String,
    },
    #[serde(rename_all = "camelCase")]
    MovingSphere {
        centre0: [f64; 3],
        centre1: [f64; 3],
        time0: f64,
        time1: f64,
        radius: f64,
        material: String,
    },
    #[serde(rename_all = "camelCase")]
    Block {
        corner0: [f64; 3],
        corner1: [f64; 3],
        material: String,
    },
    #[serde(rename_all = "camelCase")]
    Rect {
        // these must share one coordinate or it'll error out
        corner0: [f64; 3],
        corner1: [f64; 3],
        facing_forward: bool,
        material: String,
    },
    #[serde(rename_all = "camelCase")]
    Triangle {
        // forward face determined by right-hand rule
        point0: [f64; 3],
        point1: [f64; 3],
        point2: [f64; 3],
        material: String,
    },
    #[serde(rename_all = "camelCase")]
    Plane {
        point0: [f64; 3],
        point1: [f64; 3],
        point2: [f64; 3],
        uv_repeat: f64,
        material: String,
    },
    #[serde(rename_all = "camelCase")]
    Mesh {
        filename: String,
        object_name: Option<String>,
        material: String,
    },
    #[serde(rename_all = "camelCase")]
    Spotlight {
        look_from: [f64; 3],
        look_at: [f64; 3],
        length: f64,
        width: f64,
        light: [f64; 3],
    },
    #[serde(rename_all = "camelCase")]
    ConstantMedium {
        boundary: String,
        phase_function: String,
        density: f64,
    },
    #[serde(rename_all = "camelCase")]
    Translate { prototype: String, offset: [f64; 3] },
    #[serde(rename_all = "camelCase")]
    RotateX { prototype: String, degrees: f64 },
    #[serde(rename_all = "camelCase")]
    RotateY { prototype: String, degrees: f64 },
    #[serde(rename_all = "camelCase")]
    RotateZ { prototype: String, degrees: f64 },
}
