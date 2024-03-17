// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

pub mod player_control;

use crate::input::Inputs;
use crate::networking::PlayerMap;
use crate::gravity::{AtractedByGravity, GravityVector};

use bevy_gravirollback::new::*;
use bevy_gravirollback::new::for_user::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use once_cell::unsync::Lazy;
use serde::{Serialize, Deserialize};

pub const CAMERA_1ST_PERSON: Lazy<Transform> = Lazy::new(|| Transform {
    translation: Vec3::new(0.0,1.0,0.0),
    ..Transform::IDENTITY
});

/*pub const CAMERA_3RD_PERSON: Lazy<Transform> = Lazy::new(|| Transform {
    translation: Vec3::new(0.0,1.0,0.7),
    rotation: {
        let angle: f32 = -0.7;
        let (s, c) = (angle * 0.5).sin_cos();
        Quat::from_xyzw(s, 0.0, 0.0, c)
    },
    ..Transform::IDENTITY
});*/

pub const CAMERA_3RD_PERSON: Lazy<Transform> = Lazy::new(|| Transform {
    translation: Vec3::new(0.0,0.0,7.0),
    rotation: {
        let angle: f32 = 0.0;
        let (s, c) = (angle * 0.5).sin_cos();
        Quat::from_xyzw(s, 0.0, 0.0, c)
    },
    ..Transform::IDENTITY
});

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
        .register_type::<Player>()
        .register_type::<LocalPlayer>()
        .register_type::<Standing>()
        .register_type::<player_control::PlayerControl>()
        .insert_resource(player_control::PlayerControl {
            first_person: false,
            sensitivity: 0.2,
        })
        .register_type::<player_control::PlayerPhysicsConstants>()
        .init_resource::<player_control::PlayerPhysicsConstants>();
    }
}

//the player is attracted by gravity to everything, every object has gravity
//when the player stands on an object he can walk on it
//he has something like magnetic boots, player sticks to the surface he walks on
//sticking to surfaces can be made by calculating normal to the mesh triangle in contact and only allowing movement perpendicular
//to the normal, when player gets out of the mesh triangle then another one has to be found by intersecting player axis (up) with the mesh
#[derive(Component, Reflect, Default, Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct Player(pub u64);

#[derive(Component, Reflect)]
pub struct LocalPlayer;

#[derive(Component, Reflect, Clone, Copy)]
pub struct Standing(pub bool);

#[derive(Reflect, Serialize, Deserialize, Clone, Copy)]
pub struct SpawnPlayer {
    pub player: Player,
    pub rollback: RollbackID,
    pub transform: Transform,
    pub velocity: Velocity,
    pub index: Option<usize>,
}

pub fn spawn_player_system(
    mut commands: Commands,
    inputs: Res<Inputs>,
    player_query: Query<(&Player, &Transform)>,
    players: Res<PlayerMap>,
) {
    for (&player, input) in inputs.0.iter() {
        if let Some(()) = input.signals.spawn {
            if player_query.iter().find(|(&x, _)| x==player).is_none() {
                println!("spawning player {}",player.0);
        
                let rollback = players.0[&player];
                let transform = Transform::from_xyz(100.0,0.0,0.0);
                let velocity = Velocity::zero();
                let spawn = SpawnPlayer {
                    player,
                    rollback,
                    transform,
                    velocity,
                    index: None,
                };
        
                commands.add(spawn3(make_player(spawn)))
            }else{
                warn!("player {} already exists",player.0);
                continue
            }
        }
    }
    
}

pub fn make_player(event: SpawnPlayer) -> impl Fn(Option<Res<crate::networking::LocalPlayer>>, ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>, Commands) -> Entity {
    let height = 0.5;
    let radius = 0.125;

    let player_id = event.player;
    let rollback = event.rollback;
    let transform = event.transform;
    let velocity = event.velocity;

    move |local_player, mut mesh_assets, mut material_assets, mut commands| {
        let local_player = local_player.map(|x| x.0);

        //TODO: this is hacky
        let mut physics_bundle = Rollback::<crate::networking::rollback::PhysicsBundle>::default();
        let mut exists = Rollback::<Exists>::default();
        if let Some(index) = event.index {
            physics_bundle.0[index] = crate::networking::rollback::PhysicsBundle {
                transform,
                velocity,
            };
            exists.0[index] = Exists(true);
        }

        let mesh = mesh_assets      //TODO: cache mesh and material handles
            .add(Mesh::from(shape::Capsule {
                radius,
                depth: height-2.0*radius,
                ..default()
            }));
        let material = material_assets
            .add(Color::rgb(0.8, 0.7, 0.6).into());
        
        let mut player = commands.spawn((
            player_id,
            SpatialBundle {
                transform,
                ..default()
            },
            RigidBody::Dynamic,
            (physics_bundle,
            exists,
            Exists(true),),
            velocity,
            ExternalForce::default(),
            ExternalImpulse::default(),
            Damping {
                linear_damping: 0.0,
                angular_damping: 10.0,
            },
            AtractedByGravity(0.1),
            GravityVector(Vec3::ZERO),
            Standing(false),
            rollback,
            crate::networking::EntityType::Player(player_id),
        ));

        if local_player.map_or(false,|local| local==player_id) {
            player.insert((
                LocalPlayer,
                //KinematicCharacterController::default()
            ));
    
            player.with_children(|parent| {
                let mut camera = Camera3dBundle::default();
                camera.transform = *CAMERA_3RD_PERSON;
                parent.spawn(camera);
            });
        }
        
        player
        .with_children(|parent| {
            parent.spawn(PbrBundle {
                mesh,
                material,
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                ..default()
            });
    
            //for collisions with terrain
            parent.spawn((
                Collider::capsule_y((height-2.0*radius)/2.0, radius),
                Restitution {
                    coefficient: 0.0,
                    combine_rule: CoefficientCombineRule::Multiply,
                },
                Friction::coefficient(0.1),
                ColliderMassProperties::Density(1.0),
            ));
            
            //TODO: for collisions with projectiles, should correspond to mesh
            /*let scale = 4.0;
            parent.spawn((
                Collider::capsule_y((height-2.0*radius)/2.0*scale, radius*scale),
                Sensor,
                ActiveEvents::COLLISION_EVENTS,
            ));*/
        });

        player.id()
    }
}

pub fn despawn_player(player_to_despawn: Player) -> impl Fn(&mut World) {
    move |world: &mut World| {
        if let Some((entity,_)) = world.query::<(Entity, &Player)>().iter(world).find(|(_,&player)| player==player_to_despawn) {
            world.entity_mut(entity).despawn_recursive();   //TODO: maybe instead set Exists to false
        }
    }
}

/*pub fn display_events(
    mut collision_events: EventReader<CollisionEvent>,
    context: Res<RapierContext>,
    colliders: Query<&Parent,(With<Collider>,With<Sensor>)>,
    mut players: Query<&mut Standing,With<Player>>,
) {
    for collision_event in collision_events.iter() {
        //println!("Received collision event: {:?}", collision_event);

        let (e1,e2,_flags) = match collision_event {
            CollisionEvent::Started(e1,e2,f) => (e1,e2,f),
            CollisionEvent::Stopped(e1,e2,f) => (e1,e2,f),
        };

        let (player,collider) = if let Ok(x) = colliders.get(*e1) {
            (x.get(),*e1)
        }else if let Ok(x) = colliders.get(*e2) {
            (x.get(),*e2)
        }else{ continue };

        let interactions: Vec<_> = context.intersections_with(collider).collect();
        //println!("interaction {:?}",interactions);
        if let Ok(mut standing) = players.get_mut(player) {
            if interactions.iter().any(|(_e1,_e2,touches)| *touches) {
                standing.0 = true;
            }else{
                standing.0 = false;
            }
        }
    }
}*/

pub fn local_player_exists(
    query: Query<(), With<LocalPlayer>>,
) -> bool {
    !query.is_empty()
}