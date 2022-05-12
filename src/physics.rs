use bevy::prelude::*;
use heron::RigidBody;
use heron::rapier_plugin::RigidBodyHandle;
use heron::rapier_plugin::convert::IntoRapier;
use heron::rapier_plugin::rapier3d::math::Vector;
use heron::rapier_plugin::rapier3d::prelude::RigidBodySet;
use bevy_inspector_egui::Inspectable;
use bevy_inspector_egui::RegisterInspectable;

pub struct GravityPlugin;

impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app
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
    affected: Query<(&RigidBodyHandle,&AtractedByGravity)>,
    sources: Query<(&RigidBodyHandle,&CreatesGravity)>,
    mut bodies: ResMut<RigidBodySet>,
) {
    for (h1,g1) in affected.iter() {
        let mut g = Vector::zeros();
        let b1 = bodies.get(h1.into_rapier()).unwrap();
        for (h2,g2) in sources.iter() {
            let b2 = bodies.get(h2.into_rapier()).unwrap();
            let position1 = b1.mass_properties().world_com(b1.position());
            let position2 = b2.mass_properties().world_com(b2.position());

            let r_sq = (position1-position2).magnitude_squared();
            if r_sq != 0.0 {
                g += (position2 - position1).normalize() * (g2.0 * b2.mass() / r_sq);
            }
        }

        let b1 = bodies.get_mut(h1.into_rapier()).unwrap();

        b1.apply_force(g*b1.mass()*g1.0,false);
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