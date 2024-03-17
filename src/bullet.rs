// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::input::Inputs;
use crate::gravity::AtractedByGravity;
use crate::player::Player;

use bevy_gravirollback::new::for_user::*;
use bevy_gravirollback::new::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use serde::{Serialize, Deserialize};

#[derive(Component)]
pub struct Bullet;

#[derive(Reflect, Serialize, Deserialize, Clone)]
pub struct SpawnBullet {
    pub rollback: RollbackID,
    pub transform: Transform,
    pub velocity: Velocity,
    pub index: Option<usize>,
}

pub fn spawn_bullet_system(
    mut commands: Commands,
    inputs: Res<Inputs>,
    player_query: Query<(&Player, &Transform)>,
    info: Res<SnapshotInfo>,
) {
    for (&player, input) in inputs.0.iter() {
        if let Some(shoot) = &input.signals.shoot {
            if let Some((_, &transform)) = player_query.iter().find(|(&x,_)| x==player) {
                let rollback = shoot.id;
                let mut transform = transform;
                let forward = transform.forward();
                transform.translation += forward * 0.5; //TODO: move magic numbers to constants
        
                println!("spawning bullet {} player {player:?} frame {} last {}",rollback.0, info.current, info.last);
                let velocity = Velocity::linear(forward * 50.0);
                let spawn = SpawnBullet {
                    rollback,
                    transform,
                    velocity,
                    index: None,
                };
                commands.add(spawn3(make_bullet(spawn)))
            }else{
                warn!("player is shooting but is not spawned!");
            }
        }
    }
}

pub fn despawn_bullet_system(
    mut bullets: Query<(&mut Exists, &Transform, &Velocity), With<Bullet>>,
) {
    for (mut exists, transform, velocity) in bullets.iter_mut() {
        if transform.translation.length()>200.0 || velocity.linvel.length()<1.0 {
            exists.0 = false;
        }
    }
}

pub fn make_bullet(event: SpawnBullet) -> impl Fn(ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>, Commands) -> Entity {
    let radius = 0.1;

    let rollback = event.rollback;
    let transform = event.transform;
    let velocity = event.velocity;

    move |mut mesh_assets, mut material_assets, mut commands| {
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

        let mesh = mesh_assets  //TODO: cache mesh and material handles
            .add(Mesh::try_from(shape::Icosphere {
                radius,
                subdivisions: 5,
            }).unwrap());
        let material = material_assets
            .add(StandardMaterial {
                base_color: Color::RED,
                perceptual_roughness: 0.3,
                metallic: 1.0,
                ..default()
            });

        commands.spawn((
            (Bullet,
            RigidBody::Dynamic,
            Ccd::enabled(),
            SpatialBundle {
                transform,
                ..default()
            },
            physics_bundle,
            exists,
            Exists(true),
            velocity,
            AtractedByGravity(0.1),
            rollback,
            crate::networking::EntityType::Bullet,),

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
        )).with_children(|parent| {
            parent.spawn(PbrBundle {
                mesh,
                material,
                ..default()
            });
        }).id()
    }
}