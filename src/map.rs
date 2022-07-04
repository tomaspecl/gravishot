pub mod asteroid;

use bevy::prelude::*;

use bevy_pigeon::types::NetTransform;

use rand::{Rng, thread_rng};
use serde::{Serialize, Deserialize};

//This file could be made into a separate dynamicaly linked library
//which would be used as map file. User could choose the library file and 
//each one would generate different map. It would be more flexible 
//than using just gltf files or other formats to store the map data.


/// Contains all the information to construct the map.
/// Server generates this on startup or loads it from a file.
/// Server sends this to client which uses this to load the map.
/// TODO: more general maps - general meshes and objects
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Map {
    asteroids: Vec<AsteroidInstance>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AsteroidInstance {
    id: usize,
    transform: NetTransform,
}

pub fn generate_map(
    mut commands: Commands,
    assets: Res<asteroid::AsteroidAssets>
) {
    let mut asteroids = Vec::new();

    let mut rng = thread_rng();

    for _ in 0..40 {
        let transform = Transform::from_xyz(    //TODO: random orientation + scale
            rng.gen_range(-50.0..50.0),
            rng.gen_range(-50.0..50.0),
            rng.gen_range(-50.0..50.0),
        ).with_scale(Vec3::splat(5.0)).into();

        let id = rng.gen_range(0..assets.asteroids.len());

        asteroids.push(AsteroidInstance {
            id,
            transform,
        });
    }

    commands.insert_resource(Map {
        asteroids,
    });
}

pub fn load_from_map(
    mut commands: Commands,
    map: Res<Map>,
    assets: Res<asteroid::AsteroidAssets>
) {
    for asteroid in map.asteroids.iter() {
        asteroid::spawn_asteroid(asteroid.transform.into(), Some(asteroid.id), &mut commands, &assets);
    }
}