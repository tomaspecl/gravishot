use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub struct GravityPlugin;

impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(RapierConfiguration {
            gravity: Vect::ZERO,
            ..default()
        })
        .register_type::<AtractedByGravity>()
        .register_type::<CreatesGravity>()
        .register_type::<GravityVector>()
        .add_system_to_stage(CoreStage::PreUpdate,force_reset)
        .add_system(gravity_system)
        .add_system(marker_system);
    }
}

//TODO: use sparse set
#[derive(Component,Reflect,Clone)]
pub struct AtractedByGravity(pub f32);

#[derive(Component,Reflect,Clone)]
pub struct CreatesGravity(pub f32);

#[derive(Component,Reflect,Clone)]
pub struct GravityVector(pub Vec3);

fn gravity_system(
    mut affected: Query<(&RapierRigidBodyHandle,&mut ExternalForce,Option<&mut GravityVector>,&AtractedByGravity)>,
    sources: Query<(&RapierRigidBodyHandle,&CreatesGravity)>,
    context: Res<RapierContext>,
) {
    let bodies = &context.bodies;
    for (h1,mut force1,vector,g1) in affected.iter_mut() {
        let mut g = Vec3::ZERO;
        let b1 = bodies.get(h1.0).unwrap();
        for (h2,g2) in sources.iter() {
            let b2 = bodies.get(h2.0).unwrap();
            let position1 = b1.mass_properties().world_com;
            let position2 = b2.mass_properties().world_com;

            let r_sq = (position1-position2).magnitude_squared();
            if r_sq != 0.0 {
                g += Vec3::from((position2 - position1).normalize() * (g2.0 * b2.mass() / r_sq));
            }
        }

        g *= g1.0 * b1.mass();

        force1.force += g;

        if let Some(mut vector) = vector {
            vector.0 = g;
        }
    }
}

fn force_reset(
    mut forces: Query<&mut ExternalForce>,
) {
    for mut force in forces.iter_mut() {
        *force = ExternalForce::default();
    }
}

fn marker_system(
    mut commands: Commands,
    atracted: Query<(Entity,&AtractedByGravity,Option<&Children>),Without<RigidBody>>,
    creates: Query<(Entity,&CreatesGravity,Option<&Children>),Without<RigidBody>>,
) {
    for (e,g,c) in atracted.iter() {
        if let Some(c) = c {
            for child in c.iter() {
                commands.entity(*child)
                    .insert(g.clone());
            }
        }
        commands.entity(e).remove::<AtractedByGravity>();
    }

    for (e,g,c) in creates.iter() {
        if let Some(c) = c {
            for child in c.iter() {
                commands.entity(*child)
                    .insert(g.clone());
            }
        }
        commands.entity(e).remove::<CreatesGravity>();
    }
}