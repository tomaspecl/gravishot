use crate::physics::CreatesGravity;

use bevy::prelude::*;
use heron::prelude::*;

use bevy::gltf::{Gltf, GltfNode, GltfMesh, GltfPrimitive};
use heron::PendingConvexCollision;

use rand::{thread_rng,Rng};

pub struct AsteroidAssets {
    gltf: Handle<Gltf>,
    asteroids: Vec<Asteroid>,
}

struct Asteroid {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    //collider:         //TODO: cache the collider too
}

pub fn spawn_asteroid(
    transform: Transform,
    id: Option<usize>,
    commands: &mut Commands,
    asteroids: &Res<AsteroidAssets>,
) {
    //implement this https://github.com/jcornaz/heron/pull/121
    //into https://github.com/jcornaz/heron/blob/main/core/src/collision_from_mesh.rs
    //let collider_builder = heron::rapier_plugin::rapier3d::geometry::ColliderBuilder::convex_decomposition(vertices, indices);
    //heron::CollisionShape::Custom { shape: () };

    let mut rng = thread_rng();
    let asteroids = &asteroids.asteroids;
    let l = asteroids.len();
    let id = id.and_then(|x| if x<l {Some(x)}else{None}).unwrap_or(rng.gen_range(0..l));

    let a = &asteroids[id];
    let mesh = a.mesh.clone();
    let material = a.material.clone();

    commands.spawn_bundle((
        RigidBody::KinematicPositionBased,
        PendingConvexCollision::default(),  //TODO: this does not make exact collision shape but only convex hull around it
        CreatesGravity(1.0),
        transform,
        GlobalTransform::default(),
    ))
    .with_children(|p|{
        p.spawn_bundle(PbrBundle {
            mesh,
            material,
            ..Default::default()
        });
    });

    /*let scene = server.load("asteroid test.gltf#Scene0");
    commands
        .spawn_bundle((
            RigidBody::KinematicPositionBased,
            PendingConvexCollision::default(),
            CreatesGravity(1.0),
            transform,
            GlobalTransform::identity(),
            /* enable for fun
            RigidBody::Dynamic,
            crate::physics::AtractedByGravity(1.0),
            PhysicMaterial {
                restitution: 0.2,
                density: 1.0,
                friction: 0.5,
            },*/
        ))
        .with_children(|parent| {
            parent.spawn_scene(scene);
            /*
            Entity
                |--global transform
                |--transform 0,0,0
                
                Asteroid1
                    |--global transform
                    |--transform 4,0,0

                    Mesh1
                        |--global transform
                        |--transform 0,0,0
                        |--rigid body
                        |--creates gravity
                        |--etc...

                Asteroid2
                    |--global transform
                    |--transform -2.7,-1.4,-1.2

                    Mesh2
                        |--global transform
                        |--transform 0,0,0
                        |--rigid body
                        |--creates gravity
                        |--etc...
                
                Cube.001
                    |--global transform
                    |--transform 0,0,0

                    Cube.001
                        |--global transform
                        |--transform 0,0,0
                        |--rigid body
                        |--creates gravity
                        |--etc...
            */
        });*/
}

pub struct AssetsLoading(Handle<Gltf>);

pub fn start_loading(
    mut commands: Commands,
    server: Res<AssetServer>,
) {
    let handle = server.load("asteroid test.gltf");
    commands.insert_resource(AssetsLoading(handle));
}

pub fn wait_for_load(
    mut commands: Commands,
    server: Res<AssetServer>,
    handle: Option<Res<AssetsLoading>>,
    a_gltf: Res<Assets<Gltf>>,
    a_node: Res<Assets<GltfNode>>,
    a_mesh: Res<Assets<GltfMesh>>,
    mut state: ResMut<State<crate::GameState>>,
) {
    use bevy::asset::LoadState;

    if let Some(handle) = handle {
        match server.get_load_state(&handle.0) {
            LoadState::Loaded => {
                commands.remove_resource::<AssetsLoading>();
    
                let handle = handle.0.clone();
    
                let gltf = a_gltf.get(&handle).expect("asteroid assets should have been just loaded");
    
                dbg!(gltf);
    
                /*let node = gltf.named_nodes.get("Asteroid1").unwrap();
                let node = a_node.get(node).unwrap();
    
                dbg!(node);

                let gltf_mesh = node.mesh.as_ref().unwrap();
                let gltf_mesh = a_mesh.get(gltf_mesh).unwrap();

                dbg!(gltf_mesh);*/

                let mut asteroids = vec![];

                for mesh in gltf.meshes.iter() {
                    let mesh = a_mesh.get(mesh).unwrap();
                    for GltfPrimitive {mesh,material} in &mesh.primitives {
                        let mesh = mesh.clone();
                        let material = material.clone().unwrap_or_default();
                        asteroids.push(Asteroid {
                            mesh,
                            material,
                        });
                    }
                }
                
                commands.insert_resource(AsteroidAssets {
                    gltf: handle,
                    asteroids,
                });

                state.set(crate::GameState::Running).unwrap();
            },
            LoadState::Failed | LoadState::Unloaded => panic!("Could not load asteroid assets"),
            _ => ()
        }
    }
}