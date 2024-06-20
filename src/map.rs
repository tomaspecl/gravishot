// Gravishot
// Copyright (C) 2023 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

pub mod asteroid;

use bevy::prelude::*;

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
#[derive(Resource, Reflect, Serialize, Deserialize, Default, Debug, Clone)]
#[reflect(Resource)]
pub struct Map {
    asteroids: Vec<AsteroidInstance>,
}

#[derive(Reflect, Serialize, Deserialize, Default, Debug, Clone, Copy)]
pub struct AsteroidInstance {
    pub id: usize,
    pub transform: Transform,
    pub atracted_by_gravity: f32,
}
impl AsteroidInstance {
    pub fn new(transform: Option<Transform>, id: Option<usize>, asteroids: &asteroid::AsteroidAssets) -> AsteroidInstance {
        let mut rng = thread_rng();
        let l = asteroids.asteroids.len();
        let id = id.and_then(|x| if x<l {Some(x)}else{None}).unwrap_or(rng.gen_range(0..l));
        let size = 100.0;
        let transform = transform.unwrap_or(Transform::from_xyz(    //TODO: random orientation + scale
            rng.gen_range(-size..size),
            rng.gen_range(-size..size),
            rng.gen_range(-size..size),
        ).with_scale(Vec3::splat(5.0*3.0)));

        AsteroidInstance {
            id,
            transform,
            atracted_by_gravity: 0.0,
        }
    }
}

pub fn generate_map(
    mut commands: Commands,
    assets: Res<asteroid::AsteroidAssets>
) {
    let mut asteroids = Vec::new();

    for _ in 0..5 {
        asteroids.push(AsteroidInstance::new(None, Some(0), &assets));
    }
    //for _ in 0..1 {
    //    asteroids.push(AsteroidInstance::new(Some(Transform::from_scale(Vec3::new(4.0, 3.0, 4.0)*5.0*1.0)), Some(0), &assets));
    //}

    commands.insert_resource(Map {
        asteroids,
    });
}

pub fn load_from_map(
    mut commands: Commands,
    map: Res<Map>,
    assets: Res<asteroid::AsteroidAssets>,
    asteroids: Query<Entity, With<asteroid::AsteroidMarker>>,
) {
    if map.is_changed() {
        for asteroid in &asteroids {
            commands.entity(asteroid).despawn_recursive();
        }
        for &asteroid in map.asteroids.iter() {
            asteroid::spawn_asteroid(&mut commands, asteroid, &assets);
        }
    }
}