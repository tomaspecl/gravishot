use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy_inspector_egui::Inspectable;
use bevy_inspector_egui::RegisterInspectable;

pub struct GravityPlugin;

impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(RapierConfiguration {
            gravity: Vect::ZERO,
            ..default()
        })
        .register_inspectable::<AtractedByGravity>()
        .register_inspectable::<CreatesGravity>()
        .add_system(gravity_system)
        .add_system(marker_system);
    }
}

//TODO: use sparse set
#[derive(Component,Inspectable,Clone)]
pub struct AtractedByGravity(pub f32);

#[derive(Component,Inspectable,Clone)]
pub struct CreatesGravity(pub f32);

fn gravity_system(
    mut affected: Query<(&RapierRigidBodyHandle,&mut Velocity,&AtractedByGravity)>,
    sources: Query<(&RapierRigidBodyHandle,&CreatesGravity)>,
    mut context: ResMut<RapierContext>,
) {
    let bodies = &mut context.bodies;
    for (h1,mut v1,g1) in affected.iter_mut() {
        let mut g = Vec3::ZERO;
        let b1 = bodies.get(h1.0).unwrap();
        for (h2,g2) in sources.iter() {
            let b2 = bodies.get(h2.0).unwrap();
            let position1 = b1.mass_properties().world_com(b1.position());
            let position2 = b2.mass_properties().world_com(b2.position());

            let r_sq = (position1-position2).magnitude_squared();
            if r_sq != 0.0 {
                g += Vec3::from((position2 - position1).normalize() * (g2.0 * b2.mass() / r_sq));
            }
        }

        v1.linvel += g*g1.0 / 60.0;
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