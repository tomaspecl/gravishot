use crate::input::Buttons;
use crate::networking::rollback::{Rollback, Inputs};
use crate::gravity::AtractedByGravity;
use crate::networking::server::ROLLBACK_ID_COUNTER;
use crate::player::Player;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

#[derive(Component)]
pub struct Bullet;

pub struct SpawnBullet {
    pub rollback: Rollback,
    pub transform: Transform,
    pub velocity: Velocity,
}

//only on the server
pub fn spawn_bullet_system(
    inputs: Res<Inputs>,
    mut commands: Commands,
    players: Query<(&Player, &Transform)>,
) {
    for (&player,input) in inputs.0.iter() {
        if input.buttons.contains(Buttons::Shoot) {
            if let Some((_,transform)) = players.iter().find(|(&x,_)| x==player) {
                let rollback = ROLLBACK_ID_COUNTER.get_new();
                let mut transform = *transform;
                let forward = transform.forward();
                transform.translation += forward * 10.0; //TODO: move magic numbers to constants

                println!("spawning bullet {}",rollback.0);
                let velocity = Velocity::linear(forward * 10.0);
                let event = SpawnBullet {
                    rollback,
                    transform,
                    velocity,
                };
                commands.add(make_bullet(event, None));

            }else{
                warn!("player is shooting but is not spawned!");
            }
        }
    }
}

pub fn make_bullet(event: SpawnBullet, entity: Option<Entity>) -> impl Fn(&mut World) {
    let radius = 0.1;

    let rollback = event.rollback;
    let transform = event.transform;
    let velocity = event.velocity;

    move |world: &mut World| {
        let mesh = world.resource_mut::<Assets<Mesh>>() //TODO: cache mesh and material handles
            .add(Mesh::from(shape::Icosphere {
                radius,
                subdivisions: 5,
            }));
        let material = world.resource_mut::<Assets<StandardMaterial>>()
            .add(Color::rgb(1.0, 0.0, 0.0).into());

        let mut bullet = if let Some(entity) = entity {
            world.entity_mut(entity)
        }else{
            world.spawn_empty()
        };

        bullet.insert((
            Bullet,
            transform,
            RigidBody::Dynamic,
            velocity,
            AtractedByGravity(0.1),

            rollback,
            crate::networking::EntityType::Bullet,

            Collider::ball(radius),
            Restitution::coefficient(0.7),
            Friction::coefficient(0.1),
            ColliderMassProperties::Density(1.0),
            ExternalForce::default(),
            //ExternalImpulse::default(),
            Damping {
                linear_damping: 0.0,
                angular_damping: 1.0,
            },

            GlobalTransform::default(),
            ComputedVisibility::default(),
        ));
        
        bullet
        .with_children(|parent| {
            parent.spawn(PbrBundle {
                mesh,
                material,
                ..Default::default()
            });
        });
    }
}