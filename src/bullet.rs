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

pub const BULLET_DENSITY: f32 = 0.04;
pub const BULLET_VELOCITY: f32 = 
    //50.0;
    25.0;

const RADIUS: f32 = 0.075;
const MASS: f32 = 4.0/3.0*std::f32::consts::PI*RADIUS*RADIUS*RADIUS * BULLET_DENSITY;
const ANGULAR_INERTIA: f32 = 2.0/5.0*MASS*RADIUS*RADIUS;

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
    mut gun_query: Query<(&Player, &Transform, &mut Velocity, &mut crate::player::gun::Gun)>,
    info: Res<SnapshotInfo>,
) {
    for (player, &transform, mut gun_velocity, mut gun) in &mut gun_query {
        gun.0 = gun.0.saturating_sub(1);
        if gun.0!=0 {continue}

        let Some(input) = inputs.0.get(player) else{continue};
        let Some(shoot) = &input.signals.shoot else{continue};

        gun.0 = 5;

        let rollback = shoot.id;

        let mut transform = transform;
        let forward = transform.forward();
        transform.translation += forward * 0.5; //TODO: move magic numbers to constants

        println!("spawning bullet {} player {player:?} frame {} last {}",rollback.0, info.current, info.last);
        let velocity = Velocity {
            linvel: gun_velocity.linvel + forward * BULLET_VELOCITY,
            angvel: Vec3::ZERO,
        };

        let momentum = MASS * velocity.linvel;
        gun_velocity.linvel -= momentum / crate::player::gun::MASS;

        let spawn = SpawnBullet {
            rollback,
            transform,
            velocity,
            index: None,
        };
        commands.add(spawn3(make_bullet(spawn)))
    }
}

pub fn bullet_collision_system(
    mut collision_events: EventReader<CollisionEvent>,
    mut bullets: Query<(Entity, &mut Exists, &Velocity), With<Bullet>>,
    player_parts: Query<(&crate::player::Player, &crate::player::DamageCoeficient)>,
    mut players: Query<(&crate::player::Player, &mut crate::player::Health), With<crate::player::Body>>,
) {
    for event in collision_events.read() {
        let CollisionEvent::Started(e1, e2, _flags) = event else{continue};
        println!("collision event {e1:?} {e2:?}");

        let Ok((bullet, mut exists, velocity)) = bullets.get_mut(*e2) else{
            println!("e2 was not bullet");
            continue
        };

        exists.0 = false;
        
        let Ok((player,damage)) = player_parts.get(*e1) else{
            println!("collision with something else");
            continue
        };
        let Some((player, mut health)) = players.iter_mut().find(|(p,_)| **p==*player) else{
            panic!("player {player:?} part {e1:?} does not have a body");
        };

        let velocity = velocity.linvel.length();    //TODO: instead use relative velocity with respect to collided player
        let damage = damage.0*velocity;
        health.0 -= damage;

        println!("bullet {bullet:?} collision with player {player:?} part {e1:?} : velocity {velocity} damage {damage} remaining health {}",health.0);
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
            .add(Sphere::new(RADIUS));
        let material = material_assets
            .add(StandardMaterial {
                base_color: Color::RED,
                perceptual_roughness: 0.3,
                metallic: 1.0,
                ..default()
            });

        let id = commands.spawn((
            (Bullet,
            Name::new("Bullet"),
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

            Collider::ball(RADIUS),
            ActiveEvents::COLLISION_EVENTS,
            Restitution::coefficient(0.7),
            Friction::coefficient(0.1),
            //ColliderMassProperties::Density(1.0),
            AdditionalMassProperties::MassProperties(MassProperties {
                mass: MASS,
                principal_inertia: Vec3::splat(ANGULAR_INERTIA),
                ..default()
            }),
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
        }).id();
        println!("spawning bullet {id:?}");
        id
    }
}