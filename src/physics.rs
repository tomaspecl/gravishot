use bevy_gravirollback::new::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub fn physics_body_existance_system(
    mut commands: Commands,
    mut bodies: Query<(Entity, &Exists, &mut Visibility, Has<RigidBodyDisabled>), With<RigidBody>>,
) {
    for (entity, exists, mut visibility, rigid_body_disabled) in &mut bodies {
        if exists.0 {
            *visibility = Visibility::Inherited;
            if rigid_body_disabled {
                commands.entity(entity).remove::<RigidBodyDisabled>();
            }
        }else{
            *visibility = Visibility::Hidden;
            if !rigid_body_disabled {
                commands.entity(entity).insert(RigidBodyDisabled);
            }
        }
    }
}
