pub mod player_control;

use crate::input::Buttons;
use crate::networking::PlayerMap;
use crate::networking::rollback::{Rollback, Inputs};
use crate::gravity::{AtractedByGravity, GravityVector};

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use once_cell::unsync::Lazy;
use serde::{Serialize, Deserialize};

pub const CAMERA_1ST_PERSON: Lazy<Transform> = Lazy::new(|| Transform {
    translation: Vec3::new(0.0,1.0,0.0),
    ..Transform::IDENTITY
});

pub const CAMERA_3RD_PERSON: Lazy<Transform> = Lazy::new(|| Transform {
    translation: Vec3::new(0.0,1.0,0.7),
    rotation: {
        let angle: f32 = -0.7;
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
            sensitivity: 0.5,
        });
    }
}

//the player is attracted by gravity to everything, every object has gravity
//when the player stands on an object he can walk on it
//he has something like magnetic boots, player sticks to the surface he walks on
//sticking to surfaces can be made by calculating normal to the mesh triangle in contact and only allowing movement perpendicular
//to the normal, when player gets out of the mesh triangle then another one has to be found by intersecting player axis (up) with the mesh
#[derive(Component, Reflect, FromReflect, Default, Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct Player(pub u64);

#[derive(Component, Reflect)]
pub struct LocalPlayer;

#[derive(Component, Reflect, Clone, Copy)]
pub struct Standing(pub bool);

#[derive(Clone, Copy)]
pub struct SpawnPlayer {
    pub player: Player,
    pub rollback: Rollback,
    pub transform: Transform,
}

//TODO: only run on server?
pub fn spawn_player_system(
    inputs: Res<Inputs>,
    mut commands: Commands,
    players: Res<PlayerMap>,
    query: Query<&Player>,
) {
    for (&player,input) in inputs.0.iter() {
        if input.buttons.contains(Buttons::Spawn) {
            if query.iter().find(|&&x| x==player).is_none() {
                println!("spawning player {}",player.0);
                let rollback = players.0[&player];
                let transform = Transform::from_xyz(100.0,0.0,0.0);
                let event = SpawnPlayer {
                    player,
                    rollback,
                    transform,
                };
                commands.add(make_player(event, None));
            }else{
                warn!("player {} already exists",player.0);
            }
        }
    }
}

pub fn make_player(event: SpawnPlayer, entity: Option<Entity>) -> impl Fn(&mut World) {
    let height = 0.5;
    let radius = 0.125;

    let player_id = event.player;
    let rollback = event.rollback;
    let transform = event.transform;

    move |world: &mut World| {
        let local_player = world.get_resource::<crate::networking::LocalPlayer>().map(|x| x.0);
        let mesh = world.resource_mut::<Assets<Mesh>>() //TODO: cache mesh and material handles
            .add(Mesh::from(shape::Capsule {
                radius,
                depth: height-2.0*radius,
                ..Default::default()
            }));
        let material = world.resource_mut::<Assets<StandardMaterial>>()
            .add(Color::rgb(0.8, 0.7, 0.6).into());

        let mut player = if let Some(entity) = entity {
            world.entity_mut(entity)
        }else{
            world.spawn_empty()
        };

        player.insert((
            player_id,
            transform,
            RigidBody::Dynamic,
            Velocity::default(),
            ExternalForce::default(),
            //ExternalImpulse::default(),
            Damping {
                linear_damping: 0.0,
                angular_damping: 1.0,
            },
            AtractedByGravity(0.1),
            GravityVector(Vec3::ZERO),
            Standing(false),
            GlobalTransform::default(),
            ComputedVisibility::default(),
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
                ..Default::default()
            });
    
            parent.spawn((
                Collider::capsule_y((height-2.0*radius)/2.0, radius),
                Restitution::coefficient(0.7),
                Friction::coefficient(0.1),
                ColliderMassProperties::Density(1.0),
            ));
    
            let scale = 4.0;
            parent.spawn((
                Collider::capsule_y((height-2.0*radius)/2.0*scale, radius*scale),
                Sensor,
                ActiveEvents::COLLISION_EVENTS,
            ));
        });
    }
}

pub fn despawn_player (player_to_despawn: Player) -> impl Fn(&mut World) {
    move |world: &mut World| {
        if let Some((entity,_)) = world.query::<(Entity, &Player)>().iter(world).find(|(_,&player)| player==player_to_despawn) {
            world.entity_mut(entity).despawn_recursive();
        }
    }
}

pub fn stand_up(
    mut local_player: Query<(&Standing,&Transform,&GravityVector,&mut ExternalForce),With<Player>>,
) {
    for (standing,transform,vector,mut force) in local_player.iter_mut() {
        if standing.0 {
            let torque = vector.0.normalize().cross(transform.up());
            force.torque = torque * 10.0;
            //controller.up = -vector.0.normalize();
        }
    }
}

pub fn display_events(
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
        if let Ok(mut player) = players.get_mut(player) {
            if interactions.iter().any(|(_e1,_e2,touches)| *touches) {
                player.0 = true;
            }else{
                player.0 = false;
            }
        }
    }
}

pub fn local_player_exists(
    query: Query<(), With<LocalPlayer>>,
) -> bool {
    !query.is_empty()
}