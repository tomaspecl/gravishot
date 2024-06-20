use bevy_gravirollback::new::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/*
the gun is a real object (but without collisions probably)
connected to the player with a joint maybe
aiming is done by applying forces and damping to it, just like player stanging
that way the movement of the gun is realistic and player has to wait before it stabilizes
-> no instant aiming, much harder aimbots
-> player cant turn around/move and aim precisely immediately
*/

pub const MASS: f32 = 0.0001;

pub const STIFFNESS: f32 = 100.0;
pub const DUMPING: f32 = 10.0;

#[derive(Component)]
pub struct Gun(pub u8);

pub struct SpawnGun {
    pub player: Option<super::Player>,
    pub rollback_gun: RollbackID,
    pub transform: Transform,
    pub velocity: Velocity,
    pub index: Option<usize>,
}

pub fn make_gun(event: SpawnGun) -> impl Fn(ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>, Commands) -> Entity {
    let player_id = event.player;
    let rollback = event.rollback_gun;
    let transform = event.transform;
    let velocity = event.velocity;

    move |mut mesh_assets, mut material_assets, mut commands| {
        let mesh = mesh_assets.add(Cuboid::new(0.1, 0.1, 0.5));
        let material = material_assets.add(Color::rgb(0.8, 0.7, 0.6));

        let mut physics_bundle = Rollback::<crate::networking::rollback::PhysicsBundle>::default();
        let mut exists = Rollback::<Exists>::default();
        if let Some(index) = event.index {
            physics_bundle.0[index] = crate::networking::rollback::PhysicsBundle {
                transform,
                velocity,
            };
            exists.0[index] = Exists(true);   //TODO: this should not be needed, maybe only when the entity is restored
        }

        let mut transform = transform;
        transform.translation += transform.forward()*0.5;

        let mut gun = commands.spawn((
            Gun(0),
            Name::new("Gun"),
            physics_bundle,
            Exists(true),
            exists,
            rollback,
            crate::networking::EntityType::Gun(player_id),
            RigidBody::Dynamic,
            AdditionalMassProperties::MassProperties(MassProperties {
                mass: MASS,
                principal_inertia: Vec3::splat(0.00001),
                ..default()
            }),
            velocity,
            PbrBundle {
                mesh,
                material,
                transform,
                ..default()
            }
        ));

        if let Some(player) = player_id {
            gun.insert(player);
        }

        let id = gun.id();
        println!("spawning gun for player {player_id:?} entity {id:?}");
        id
    }
}

pub fn connect_joints(
    gun_no_joint: Query<(Entity, &super::Player), (With<Gun>, Without<ImpulseJoint>)>,
    gun_no_player: Query<Entity, (With<Gun>, With<ImpulseJoint>, Without<super::Player>)>,
    mut players: Query<(Entity, &super::Player, &mut super::PlayerParts), With<super::Body>>,
    mut commands: Commands,
) {
    for (e,player) in &gun_no_joint {
        //TODO: this will crash when player1 joins and exists, then player2 joins -> player2 will receive State(EntityType::Gun(Player(1))) even when it should be EntityType(None)
        //EntityType should not contain that information as it is duplicit
        let (entity,_,mut parts) = players.iter_mut().find(|(_,p,_)| **p==*player).expect("the player holding this gun should exist");

        let joint = SphericalJointBuilder::new()
            .local_anchor1(Vec3::new(0.0,0.0,0.0))
            .local_anchor2(Vec3::new(0.0,0.0,0.5))
            .limits(JointAxis::AngZ, [0.0, 0.0])
            .limits(JointAxis::AngX, [-0.4, 0.4])
            .limits(JointAxis::AngY, [-0.4, 0.4])
            .motor_position(JointAxis::AngX, 0.0, STIFFNESS, DUMPING)
            .motor_position(JointAxis::AngY, 0.0, STIFFNESS, DUMPING);
        //let joint = FixedJointBuilder::new()
        //    .local_anchor1(Vec3::new(0.0,0.0,0.0))
        //    .local_anchor2(Vec3::new(0.0,0.0,0.5));
        let joint = ImpulseJoint::new(entity, joint);

        commands.entity(e).insert(joint);
        parts.gun = e;

    }
    
    for e in &gun_no_player {
        commands.entity(e).remove::<ImpulseJoint>();
    }
}

pub fn update_joints(
    mut gun: Query<(&super::Player, &mut ImpulseJoint), With<Gun>>,
    head: Query<(&super::Player, &Transform), With<super::Head>>,
    constants: Res<super::player_control::PlayerPhysicsConstants>,
) {
    let stiffness = constants.gun_stiffness;
    let damping = constants.gun_damping;

    for (&player, mut joint) in &mut gun {
        let Some((_,&head)) = head.iter().find(|&(&player2,_)| player2==player) else{continue};

        joint.data
            .set_local_basis1(head.rotation)
            .set_motor_position(JointAxis::AngX, 0.0, stiffness, damping)
            .set_motor_position(JointAxis::AngY, 0.0, stiffness, damping);
    }
}