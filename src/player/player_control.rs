// Gravishot
// Copyright (C) 2023 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::networking::rollback::Inputs;
use crate::input::{Buttons, MOUSE_SCALE};

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

pub fn center_cursor(mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    window.set_cursor_position(Vec2::new(window.width()/2.0,window.height()/2.0));
}

pub fn change_player_control(
    input: Res<Input<MouseButton>>,
    mut windows: ResMut<Windows>,
    mut control: ResMut<PlayerControl>,
    player: Query<Entity, With<super::LocalPlayer>>,
    mut camera: Query<(&mut Transform, &Parent), With<Camera>>,
) {
    use bevy::window::CursorGrabMode;
    let window = windows.primary_mut();
    
    if input.just_pressed(MouseButton::Middle) {
        control.first_person = !control.first_person;
        if control.first_person {
            window.set_cursor_grab_mode(CursorGrabMode::Locked);
        }else{
            window.set_cursor_grab_mode(CursorGrabMode::None);
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

pub fn movement_system(
    inputs: Res<Inputs>,
    mut query: Query<(&super::Player, &mut Transform, &mut Velocity, &super::Standing)>,
) {
    for (player,mut transform,mut velocity,&_standing) in query.iter_mut() {
        let Some(input) = inputs.0.get(player) else{continue};
        
        let b = input.buttons;
        let m = &input.mouse;

        let mut t = Vec3::ZERO;
        let mut r = Vec3::ZERO;

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
            Buttons::Q        => r += Vec3::Z;
            Buttons::E        => r += -Vec3::Z;
        }
    
        for &(x,y) in m.deltas.iter() {
            r += Vec3::new(x as f32 / MOUSE_SCALE,y as f32 / MOUSE_SCALE,0.0) * -0.1
        }

        let rot = transform.rotation;
    
        let translation_coefficient = 0.1;
        let rotation_coefficient = 0.1;
    
        let mov = (rot * t) * translation_coefficient;
        /*if standing.0 {   //TODO: implement snap to ground
            if mov==Vec3::ZERO {
                //controller.translation = None;
            }else{
                //controller.translation = Some(mov);
            }
        }else{*/
            velocity.linvel += mov;
        //}
        
        let rot = r * rotation_coefficient;
    
        transform.rotation *= Quat::from_euler(EulerRot::YXZ,rot.x,rot.y,rot.z);
    }
}

pub fn read_result_system(controllers: Query<&KinematicCharacterControllerOutput>) {
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