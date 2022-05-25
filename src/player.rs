use crate::physics::AtractedByGravity;

use bevy::prelude::*;
use heron::prelude::*;

use bevy::input::{
    keyboard::KeyboardInput,
    mouse::MouseMotion,
};

use bevy_pigeon::sync::{NetComp, NetEntity, CNetDir, SNetDir};
use bevy_pigeon::types::NetTransform;
use carrier_pigeon::CId;
use carrier_pigeon::net::CIdSpec;

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
        .spawn_bundle((
            Player {
                cid,
            },
            transform,
            RigidBody::Dynamic,
            Velocity::default(),
            Damping::from_angular(1.0),
            PhysicMaterial {
                restitution: 0.7,
                density: 1.0,
                friction: 0.1,
            },
            AtractedByGravity(0.1),
            //PendingConvexCollision::default(),
            GlobalTransform::default(),
            NetEntity::new(nid),
            crate::networking::NetMarker::Player,
        ));
        
        if cid==local_player {
            player.insert_bundle((
                LocalPlayer,
                NetComp::<Transform, NetTransform>::new(true,
                    CNetDir::To,
                    SNetDir::To(CIdSpec::All)
                )
            ));

            player.with_children(|parent| {
                let mut camera = PerspectiveCameraBundle::new_3d();
                //let mut camera = OrthographicCameraBundle::new_3d();
                camera.transform = Transform::from_xyz(0.0,1.0,0.7).mul_transform(Transform::from_rotation(Quat::from_rotation_x(-0.7)));
                parent.spawn_bundle(camera);
            });
        }else{
            player.insert(NetComp::<Transform, NetTransform>::new(true,CNetDir::From,SNetDir::ToFrom(CIdSpec::Except(cid),CIdSpec::Only(cid))));
        }
        
        player
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(bevy::prelude::shape::Capsule {
                    radius,
                    depth: height-2.0*radius,
                    ..Default::default()
                })),
                material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                ..Default::default()
            })
            .insert(CollisionShape::Capsule { half_segment: (height-2.0*radius)/2.0, radius });
        });
    }
}

pub fn movement_system(
    mut query: Query<(&mut Transform, &mut Velocity), With<LocalPlayer>>,
    mut keyboard: EventReader<KeyboardInput>,
    mut mouse: EventReader<MouseMotion>,
    mouse_button: Res<Input<MouseButton>>,
) {
    for (mut transform, mut velocity) in query.iter_mut() {
        let rot = transform.rotation;

        let mut t = Vec3::ZERO;
        let mut r = Vec3::ZERO;
    
        for key in keyboard.iter() {
            t += match key.key_code {
                Some(KeyCode::W) => -Vec3::Z,
                Some(KeyCode::S) => Vec3::Z,
                Some(KeyCode::A) => -Vec3::X,
                Some(KeyCode::D) => Vec3::X,
                Some(KeyCode::Space) => Vec3::Y,
                Some(KeyCode::LShift) => -Vec3::Y,
                _ => Vec3::ZERO,
            };
    
            r += match key.key_code {
                Some(KeyCode::Q) => Vec3::Z,
                Some(KeyCode::E) => -Vec3::Z,
                _ => Vec3::ZERO,
            };
        }
    
        if mouse_button.pressed(MouseButton::Left) {
            for mouse_motion in mouse.iter() {
                r += Vec3::new(mouse_motion.delta.x,mouse_motion.delta.y,0.0) * -0.1
            }
        }
    
        let translation_coefficient = 0.1;
        let rotation_coefficient = 0.1;
    
        velocity.linear += (rot * t) * translation_coefficient;
        let rot = r * rotation_coefficient;
    
        transform.rotation *= Quat::from_euler(EulerRot::YXZ,rot.x,rot.y,rot.z);
    }
}

pub fn local_player_exists(
    query: Query<(), With<LocalPlayer>>,
) -> bool {
    !query.is_empty()
}