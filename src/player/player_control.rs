// Gravishot
// Copyright (C) 2023 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::gravity::GravityVector;
use crate::input::{Buttons, Inputs, MOUSE_SCALE};

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
pub struct PlayerControl {
    pub first_person: bool,
    pub sensitivity: f32,
}

pub fn is_first_person(control: Res<PlayerControl>) -> bool {
    control.first_person
}

pub fn center_cursor(mut window_query: Query<&mut Window, With<bevy::window::PrimaryWindow>>) {
    let mut window = window_query.single_mut();
    let pos = Vec2::new(window.width()/2.0,window.height()/2.0);
    window.set_cursor_position(Some(pos));
}

pub fn change_player_control(
    input: Res<Input<MouseButton>>,
    mut window_query: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
    mut control: ResMut<PlayerControl>,
    player: Query<Entity, With<super::LocalPlayer>>,
    mut camera: Query<(&mut Transform, &Parent), With<Camera>>,
) {
    use bevy::window::CursorGrabMode;
    let mut window = window_query.single_mut();
    
    if input.just_pressed(MouseButton::Middle) {
        control.first_person = !control.first_person;
        if control.first_person {
            window.cursor.grab_mode = CursorGrabMode::Locked;
        }else{
            window.cursor.grab_mode = CursorGrabMode::None;
        }
        let player = player.get_single().unwrap();
        for (mut transform,parent) in camera.iter_mut() {
            if parent.get()==player {
                *transform = if control.first_person {
                    *super::CAMERA_1ST_PERSON
                }else{
                    *super::CAMERA_3RD_PERSON
                };
            }
        }
    }
}

//for changing during runtime using bevy_inspector_egui
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct PlayerPhysicsConstants {
    lmax: f32,
    l0: f32,
    lmaxwalk: f32,
    legs_power: f32,
    legs_const: f32,
    legs_torque: f32,
    legs_torque_max: f32,
    distance_stand_up: f32,
    distance_free: f32,
    jump_impulse: f32,
    linear_damping_still: f32,
    linear_damping_walking: f32,
}
impl Default for PlayerPhysicsConstants {
    fn default() -> Self {
        Self {
            lmax: 1.0,
            l0: 0.7,
            lmaxwalk: 1.0,
            legs_power: 0.3,
            legs_const: 1.0,
            legs_torque: 0.1,
            legs_torque_max: 0.1,
            distance_stand_up: 0.35,
            distance_free: 1.2,
            jump_impulse: 0.15,
            linear_damping_still: 5.0,
            linear_damping_walking: 0.5,
        }
    }
}

pub fn movement_system(
    inputs: Res<Inputs>,
    mut query: Query<(Entity, &super::Player, &mut Transform, /*&mut Velocity,*/ &GravityVector, &mut ExternalForce, &mut ExternalImpulse, &mut super::Standing, &mut Damping)>,
    rapier_context: Res<RapierContext>,
    constants: Res<PlayerPhysicsConstants>,
    keyboard: Res<bevy::input::Input<KeyCode>>,
    mut jetpack: Local<bool>,
) {
    //temporary hack
    if keyboard.just_pressed(KeyCode::J) {
        *jetpack = !*jetpack;
        println!("jetpack {}",*jetpack);
    }

    for (
        player_entity,
        player,
        mut transform,
        //mut velocity,
        &gravity,
        mut force,
        mut impulse,
        mut standing,
        mut damping,
    ) in query.iter_mut() {
        let mut t = Vec3::ZERO;
        let mut r = Vec3::ZERO;

        if let Some(input) = inputs.0.get(player) {
            let b = input.buttons;
            let m = &input.mouse;

            use crate::input::pressed;
            pressed! {(b);
                //translation
                Buttons::W        => t += -Vec3::Z;
                Buttons::S        => t += Vec3::Z;
                Buttons::A        => t += -Vec3::X;
                Buttons::D        => t += Vec3::X;
                Buttons::Space    => t += Vec3::Y;
                Buttons::Shift    => t += -Vec3::Y;
                //rotation
                Buttons::Q        => r += Vec3::Z * 0.1;
                Buttons::E        => r += -Vec3::Z * 0.1;
            }

            for &(x,y) in m.deltas.iter() {
                r += Vec3::new(x as f32 / MOUSE_SCALE,y as f32 / MOUSE_SCALE,0.0) * -0.1
            }
        }

        let translation_coefficient = 0.1;
        let rotation_coefficient = 0.1;
        
        let rot = r * rotation_coefficient;
        transform.rotation *= Quat::from_euler(EulerRot::YXZ,rot.x,rot.y,rot.z);    //TODO: use force/inpulse instead? maybe just for Q/E

        let mut rot = transform.rotation;

        let ray_pos = transform.translation;
        let ray_dir = gravity.0.normalize();
        let max_toi = 10.0;
        let solid = true;
        let filter = QueryFilter::new().exclude_rigid_body(player_entity);
        
        /*
        IDEA: player will have a base (legs)
        the base will be a KinematicBodyController
        The rest of the player body will be attached to the base
        The rest of the body will be RigidBody::Dynamic

        OR the body is RigidBody::Dynamic when free floating
        and RigidBody::KinematicVelocityBased when standing

        OR it can be RigidBody::Dynamic all the time, without magnetic boots
        lock the rotation axis so that bumps can not start turning the player
        -> player rotation is controlled solely by the player
        
        OR main body RigidBody::Dynamic with high mass
        and lower body (legs) with slight magnetic boots effect (stickyness to surfaces)
        lower body is also RigidBody::Dynamic
        main and lower is connected by a link (Joint), like a spring (up/down motion mainly)
        lower body collision decides if its standing
        when standing it can move and jump
        this should model human body better

        OR (currently implemented) main player body hovers above ground
        held in place by forces simulating legs
        -> completely Dynamic RigidBody system
        -> responds to forces, impulses, etc...
        */

        let _lmax = constants.lmax;
        let _lmaxwalk = constants.lmaxwalk;
        let l0 = constants.l0;
        let legs_power = constants.legs_power;
        let legs_const = constants.legs_const;
        let legs_torque = constants.legs_torque;
        let legs_torque_max = constants.legs_torque_max;

        let distance_stand_up = constants.distance_stand_up;
        let distance_free = constants.distance_free;
        let jump_impulse = constants.jump_impulse;
        let linear_damping_still = constants.linear_damping_still;
        let linear_damping_walking = constants.linear_damping_walking;

        if let Some((_ground_entity, intersection)) = rapier_context.cast_ray_and_get_normal(
            ray_pos, ray_dir, max_toi, solid, filter
        ) {
            let distance = intersection.toi;
            let _ground = intersection.point;
            let ground_up = intersection.normal;
            //println!("Entity {:?} hit at point {} with normal {}", ground_entity, ground, ground_normal);

            let mut torque = ray_dir.cross(transform.up()) / distance.max(1.0);
            torque = torque.clamp_length_max(legs_torque_max);
            force.torque = torque  * legs_torque;

            //println!("player {} ground distance {distance} torque {}",player.0,torque.length());

            if distance<distance_stand_up {
                standing.0 = true;
            }
            if distance>distance_free {
                standing.0 = false;
            }

            if standing.0 {
                if t.length_squared()==0.0 {
                    damping.linear_damping = linear_damping_still;
                }else{
                    damping.linear_damping = linear_damping_walking;
                }
                

                let d = distance-l0;
                let legs_force = -d.signum()*d.abs().powf(legs_power)*legs_const;
                //println!("legs force {legs_force}");
                force.force += -ray_dir*legs_force;
            }else{
                damping.linear_damping = 0.0;
            }

            /*if distance<lmax {
                let d = distance-l0;
                let legs_force = -d.signum()*d.abs().powf(legs_power)*legs_const;
                println!("legs force {legs_force}");
                force.force += transform.up()*legs_force;
            }*/

            if standing.0 && distance>distance_stand_up {
                /*let ground_right = transform.forward()
                    .cross(ground_up)
                    .try_normalize()
                    .unwrap_or(transform.right());
                let ground_back = ground_right
                    .cross(ground_up)
                    .normalize();*/
                let ground_back = transform.right().cross(ground_up)
                    .try_normalize()
                    .unwrap_or_else(|| ground_up.any_orthonormal_vector());
                let ground_right = ground_up.cross(ground_back).normalize();
                rot = Quat::from_mat3(&Mat3::from_cols(ground_right, ground_up, ground_back));
                //rot = Quat::from_mat3(&Mat3::from_cols(ground_right, transform.up(), ground_back));

                let jump = t.y.max(0.0);
                if jump!=0.0 {
                    standing.0 = false;
                    let jump = (rot * Vec3::new(0.0, jump, 0.0)) * jump_impulse;
                    impulse.impulse += jump;
                    println!("jump {}",jump.length());
                }
                t.y = 0.0;
                let mov = (rot * t) * translation_coefficient;
                force.force += mov;
            }
        }

        if *jetpack {
            let mov = (rot * t) * translation_coefficient;
            force.force += mov;
        }

        //TODO: use this to toggle standing?
        //for contact in rapier_context.contacts_with(player_entity) {}
    }
}

pub fn read_result_system(controllers: Query<&KinematicCharacterControllerOutput>) {    //TODO: this is not used
    let p = match controllers.get_single() {
        Ok(p) => p,
        Err(_) => return,
    };
    let KinematicCharacterControllerOutput {
        grounded,
        desired_translation,
        effective_translation,
        collisions,
    } = p;
    println!("Player moved by {effective_translation:?}, wanted move {desired_translation:?} and touches the ground: {grounded:?}");
    dbg!(collisions);
}