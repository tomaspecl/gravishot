// Gravishot
// Copyright (C) 2023 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
                transform.translation += forward * 0.5; //TODO: move magic numbers to constants

                println!("spawning bullet {}",rollback.0);
                let velocity = Velocity::linear(forward * 50.0);
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

//only on the server
pub fn despawn_bullet_system(
    mut commands: Commands,
    bullets: Query<(Entity, &Transform, &Velocity), With<Bullet>>,
) {
    for (bullet,transform,velocity) in bullets.iter() {
        if transform.translation.length()>200.0 || velocity.linvel.length()<1.0 {
            commands.entity(bullet).despawn_recursive();
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
            .add(Mesh::try_from(shape::Icosphere {
                radius,
                subdivisions: 5,
            }).unwrap());
        let material = world.resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::RED,
                perceptual_roughness: 0.3,
                metallic: 1.0,
                ..default()
            });

        let mut bullet = if let Some(entity) = entity {
            world.entity_mut(entity)
        }else{
            world.spawn_empty()
        };

        //println!("spawning bullet {:?} rollback {}",bullet.id(),rollback.0);

        bullet.insert((
            Bullet,
            RigidBody::Dynamic,
            Ccd::enabled(),
            velocity,
            AtractedByGravity(0.1),
            SpatialBundle {
                transform,
                ..default()
            },
            
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
        ));
        
        bullet
        .with_children(|parent| {
            parent.spawn(PbrBundle {
                mesh,
                material,
                ..default()
            });
        });
    }
}