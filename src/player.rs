use crate::physics::{AtractedByGravity, GravityVector};

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy::input::mouse::MouseMotion;

use bevy_pigeon::sync::{NetComp, NetEntity, CNetDir, SNetDir};
use bevy_pigeon::types::NetTransform;
use carrier_pigeon::CId;
use carrier_pigeon::net::CIdSpec;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_event::<SpawnPlayerEvent>()
        .add_event::<DespawnPlayerEvent>()
        .register_type::<Standing>();
    }
}

//the player is attracted by gravity to everything, every object has gravity
//when the player stands on an object he can walk on it
//he has something like magnetic boots, player sticks to the surface he walks on
//sticking to surfaces can be made by calculating normal to the mesh triangle in contact and only allowing movement perpendicular
//to the normal, when player gets out of the mesh triangle then another one has to be found by intersecting player axis (up) with the mesh
#[derive(Component)]
pub struct Player {
    pub cid: CId,
}

#[derive(Component)]
pub struct LocalPlayer;

#[derive(Component,Reflect)]
pub struct Standing(pub bool);

#[derive(Clone)]
pub struct SpawnPlayerEvent {
    pub cid: CId,
    pub nid: u64,
    pub transform: Transform,
}

impl From<crate::networking::SpawnPlayer> for SpawnPlayerEvent {
    fn from(p: crate::networking::SpawnPlayer) -> Self {
        Self {
            cid: p.cid,
            nid: p.nid,
            transform: p.transform.into(),
        }
    }
}

pub fn spawn_player_event_handler(
    mut event: EventReader<SpawnPlayerEvent>,
    netconfig: Res<crate::networking::NetConfig>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let local_player = netconfig.local_player_cid;
    for event in event.iter() {
        let cid = event.cid;
        let nid = event.nid;
        let transform = event.transform;

        let height = 0.5;
        let radius = 0.125;
        let mut player = commands
        .spawn((
            Player {
                cid,
            },
            transform,
            RigidBody::Dynamic,
            Velocity::default(),
            ExternalForce::default(),
            ExternalImpulse::default(),
            Damping {
                linear_damping: 0.0,
                angular_damping: 1.0,
            },
            AtractedByGravity(0.1),
            GravityVector(Vec3::ZERO),
            Standing(false),
            GlobalTransform::default(),
            ComputedVisibility::default(),
            NetEntity::new(nid),
            crate::networking::NetMarker::Player,
        ));
        
        if cid==local_player {
            player.insert((
                LocalPlayer,
                NetComp::<Transform, NetTransform>::new(true,
                    CNetDir::To,
                    SNetDir::To(CIdSpec::All)
                )
            ));

            player.with_children(|parent| {
                let mut camera = Camera3dBundle::default();
                //let mut camera = OrthographicCameraBundle::new_3d();
                camera.transform = Transform::from_xyz(0.0,1.0,0.7).mul_transform(Transform::from_rotation(Quat::from_rotation_x(-0.7)));
                parent.spawn(camera);
            });
        }else{
            player.insert(NetComp::<Transform, NetTransform>::new(true,CNetDir::From,SNetDir::ToFrom(CIdSpec::Except(cid),CIdSpec::Only(cid))));
        }
        
        player
        .with_children(|parent| {
            parent.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(bevy::prelude::shape::Capsule {
                    radius,
                    depth: height-2.0*radius,
                    ..Default::default()
                })),
                material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
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

pub fn stand_up(
    mut local_player: Query<(&Standing,&Transform,&GravityVector,&mut ExternalForce),With<LocalPlayer>>,
) {
    for (standing,transform,vector,mut force) in local_player.iter_mut() {
        if standing.0 {
            let torque = vector.0.normalize().cross(transform.up());
            force.torque = torque * 10.0;
        }
    }
}

pub fn display_events(
    mut collision_events: EventReader<CollisionEvent>,
    context: Res<RapierContext>,
    colliders: Query<&Parent,(With<Collider>,With<Sensor>)>,
    mut players: Query<&mut Standing,With<LocalPlayer>>,
) {
    for collision_event in collision_events.iter() {
        println!("Received collision event: {:?}", collision_event);

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
        println!("{:?}",interactions);
        if let Ok(mut player) = players.get_mut(player) {
            if interactions.iter().any(|(_e1,_e2,touches)| *touches) {
                player.0 = true;
            }else{
                player.0 = false;
            }
        }
    }
}

pub struct DespawnPlayerEvent(pub CId);

pub fn despawn_player_event_handler(
    mut event: EventReader<DespawnPlayerEvent>,
    mut commands: Commands,
    players: Query<(Entity,&Player)>,
) {
    for event in event.iter() {
        for (entity,player) in players.iter() {
            if player.cid==event.0 {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

pub fn movement_system(
    mut query: Query<(&mut Transform, &mut Velocity), With<LocalPlayer>>,
    keyboard: Res<Input<KeyCode>>,
    mut mouse: EventReader<MouseMotion>,
    mouse_button: Res<Input<MouseButton>>,
) {
    let mut t = Vec3::ZERO;
    let mut r = Vec3::ZERO;

    for key in keyboard.get_pressed() {
        t += match key {
            KeyCode::W => -Vec3::Z,
            KeyCode::S => Vec3::Z,
            KeyCode::A => -Vec3::X,
            KeyCode::D => Vec3::X,
            KeyCode::Space => Vec3::Y,
            KeyCode::LShift => -Vec3::Y,
            _ => Vec3::ZERO,
        };

        r += match key {
            KeyCode::Q => Vec3::Z,
            KeyCode::E => -Vec3::Z,
            _ => Vec3::ZERO,
        };
    }

    if mouse_button.pressed(MouseButton::Left) {
        for mouse_motion in mouse.iter() {
            r += Vec3::new(mouse_motion.delta.x,mouse_motion.delta.y,0.0) * -0.1
        }
    }

    for (mut transform, mut velocity) in query.iter_mut() {
        let rot = transform.rotation;
    
        let translation_coefficient = 0.1;
        let rotation_coefficient = 0.1;
    
        velocity.linvel += (rot * t) * translation_coefficient;
        let rot = r * rotation_coefficient;
    
        transform.rotation *= Quat::from_euler(EulerRot::YXZ,rot.x,rot.y,rot.z);
    }
}

pub fn local_player_exists(
    query: Query<(), With<LocalPlayer>>,
) -> bool {
    !query.is_empty()
}