// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

pub mod player_control;
pub mod gun;

use crate::input::Inputs;
use crate::gravity::{AtractedByGravity, GravityVector};

use bevy_gravirollback::new::*;
use bevy_gravirollback::new::for_user::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use once_cell::unsync::Lazy;
use serde::{Serialize, Deserialize};

#[derive(Component)]
pub struct CameraType {
    pub first_person: bool,
}

pub const CAMERA_1ST_PERSON: Lazy<Transform> = Lazy::new(|| Transform {
    translation: Vec3::new(0.0,0.0,0.0),
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
    translation: Vec3::new(0.0,5.0,4.0),
    rotation: {
        let angle: f32 = -0.8;
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
        .register_type::<Health>()
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

#[derive(Component, Reflect)]
pub struct Standing(pub bool);

#[derive(Component, Reflect)]
pub struct PlayerParts {
    pub head: Entity,
    pub gun: Entity,
}

#[derive(Component)]
pub struct Body;

#[derive(Component)]
pub struct Head;

#[derive(Default, Reflect, Serialize, Deserialize, Clone, Debug)]
pub struct HeadData {
    pub rotation: Quat,
}
impl RollbackCapable for HeadData {
    type RestoreQuery<'a> = &'a PlayerParts;
    type RestoreExtraParam<'a> = Query<'a,'a, &'static mut Transform, With<Head>>;
    type SaveQuery<'a> = &'a PlayerParts;
    type SaveExtraParam<'a> = Query<'a,'a, &'static Transform, With<Head>>;

    fn restore(&self, q: &PlayerParts, extra: &mut bevy::ecs::system::StaticSystemParam<Self::RestoreExtraParam<'_>>) {
        extra.get_mut(q.head).unwrap().rotation = self.rotation;
    }

    fn save(q: &PlayerParts, extra: &mut bevy::ecs::system::StaticSystemParam<Self::SaveExtraParam<'_>>) -> Self {
        HeadData {
            rotation: extra.get(q.head).unwrap().rotation,
        }
    }
}

const PLAYER_HEALTH: Health = Health(100.0);
#[derive(Component, Reflect, Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Health(pub f32);

#[derive(Component, Reflect)]
pub struct DamageCoeficient(pub f32);

#[derive(Reflect, Serialize, Deserialize, Clone)]
pub struct SpawnPlayer {
    pub player: Player,
    pub rollback_body: RollbackID,
    pub transform: Transform,
    pub velocity: Velocity,
    pub index: Option<usize>,
    pub head_data: HeadData,
    pub health: Health,
}

pub fn spawn_player_system(
    mut commands: Commands,
    inputs: Res<Inputs>,
    player_query: Query<(&Player, &Transform), With<Body>>,
) {
    for (&player, input) in inputs.0.iter() {
        if let Some(spawn) = &input.signals.spawn {
            if player_query.iter().find(|(&x, _)| x==player).is_none() {
                let rollback_body = spawn.body;
                let rollback_gun = spawn.gun;
                let transform = Transform::from_xyz(100.0,0.0,0.0);
                let velocity = Velocity::zero();
                let spawn = SpawnPlayer {
                    player,
                    rollback_body,
                    transform,
                    velocity,
                    index: None,
                    head_data: HeadData::default(),
                    health: PLAYER_HEALTH,
                };
        
                commands.add(spawn3(make_player(spawn)));

                let spawn = gun::SpawnGun {
                    player: Some(player),
                    rollback_gun,
                    transform,
                    velocity,
                    index: None,
                };
        
                commands.add(spawn3(gun::make_gun(spawn)));
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
    let rollback_body = event.rollback_body;
    let transform = event.transform;
    let velocity = event.velocity;
    let head_data = event.head_data;
    let health = event.health;

    move |local_player, mut mesh_assets, mut material_assets, mut commands| {
        let local_player = local_player.map(|x| x.0);
        
        //TODO: this is hacky
        let mut physics_bundle = Rollback::<crate::networking::rollback::PhysicsBundle>::default();
        let mut head_data_storage = Rollback::<HeadData>::default();
        let mut exists = Rollback::<Exists>::default();
        let mut health_rb = Rollback::<Health>::default();
        if let Some(index) = event.index {
            physics_bundle.0[index] = crate::networking::rollback::PhysicsBundle {
                transform,
                velocity,
            };
            head_data_storage.0[index] = head_data.clone();
            exists.0[index] = Exists(true);   //TODO: this should not be needed, maybe only when the entity is restored
            health_rb.0[index] = health;
        }

        //TODO: cache mesh and material handles
        let mesh = mesh_assets.add(Capsule3d::new(radius, height-2.0*radius));
        let head_mesh = mesh_assets.add(Sphere::new(radius));
        let material = material_assets.add(Color::rgb(0.8, 0.7, 0.6));
        
        let mut player = commands.spawn((
            player_id,
            SpatialBundle {
                transform,
                ..default()
            },
            RigidBody::Dynamic,
            (
                physics_bundle,
                head_data_storage,
                exists,
                Exists(true),
            ),
            velocity,
            ExternalForce::default(),
            ExternalImpulse::default(),
            Damping {
                linear_damping: 0.0,
                angular_damping: 1.0,
            },
            AtractedByGravity(0.1),
            GravityVector(Vec3::ZERO),
            Standing(false),
            (health,
            health_rb,
            DamageCoeficient(1.0),),
            rollback_body,
            (Body,
            Name::new("Player Body"),),
            crate::networking::EntityType::Player(player_id),
        ));

        let is_local = local_player.map_or(false,|local| local==player_id);
        if is_local {
            player.insert((
                LocalPlayer,
                //KinematicCharacterController::default()
            ));

            player.with_children(|parent| {
                parent.spawn((
                    Camera3dBundle {
                        transform: *CAMERA_3RD_PERSON,
                        ..default()
                    },
                    CameraType {
                        first_person: false,
                    },
                ));
            });
        }
        
        let mut parts = PlayerParts {
            head: Entity::PLACEHOLDER,
            gun: Entity::PLACEHOLDER,
        };

        player.with_children(|parent| {
            parent.spawn(PbrBundle {
                mesh,
                material: material.clone(),
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                ..default()
            });

            //TODO: have multiple cameras and switch between them
            //player head, move to head.rs ?
            let mut head = parent.spawn((
                Head,
                Name::new("Player Head"),
                DamageCoeficient(5.0),  //TODO: head damage does not work without a Collider
                player_id,
                PbrBundle {
                    mesh: head_mesh,
                    material,
                    transform: Transform::from_xyz(0.0, height, 0.0),
                    ..default()
                },
            ));
            parts.head = head.id();
            if is_local {
                head.with_children(|parent| {
                    parent.spawn((
                        Camera3dBundle {
                            transform: *CAMERA_1ST_PERSON,
                            camera: Camera {
                                is_active: false,
                                ..default()
                            },
                            ..default()
                        },
                        CameraType {
                            first_person: true,
                        },
                    ));
                });
            }
    
            
            
            //TODO: for collisions with projectiles, should correspond to mesh
            /*let scale = 4.0;
            parent.spawn((
                Collider::capsule_y((height-2.0*radius)/2.0*scale, radius*scale),
                Sensor,
                ActiveEvents::COLLISION_EVENTS,
            ));*/
        });

        //for collisions with terrain
        //parent.spawn((
        player.insert((
            Collider::capsule_y((height-2.0*radius)/2.0, radius),
            Restitution {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Multiply,
            },
            Friction::coefficient(0.1),
            ColliderMassProperties::Density(1.0),
        ));

        player.insert(parts);
        let id = player.id();

        println!("spawning player {player_id:?} entity {id:?}");

        id
    }
}

//TODO: this does not always work, when the client is not fully synced it will miss a hit and then not remove the Gun's Player
//this will cause a crash
pub fn health_system(
    mut players: Query<(&PlayerParts, &mut Exists, &Health)>,
    mut commands: Commands,
) {
    for (parts,mut exists,hp) in &mut players {
        if hp.0 <= 0.0 {
            exists.0 = false;
            if parts.gun != Entity::PLACEHOLDER {
                commands.entity(parts.gun).remove::<Player>();
            }
        }
    }
}

pub fn despawn_player(player_to_despawn: Player) -> impl Fn(&mut World) {
    move |world: &mut World| {
        //TODO: this does not work, this is not used when rollback systems despawn the player, so the gun is not despawned
        //rollback despawn system need some callback that despawns even the gun
        //or just put RollbackID onto the gun
        let entities = world.query_filtered::<(Entity, &Player), With<Body>>().iter(world).filter_map(|(entity,&player)| if player==player_to_despawn {Some(entity)}else{None}).collect::<Vec<_>>();
        for e in entities {
            world.get_entity_mut(e).map(|e| e.despawn_recursive()); //TODO: maybe instead set Exists to false
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