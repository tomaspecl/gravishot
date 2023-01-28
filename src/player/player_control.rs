use crate::player::LocalPlayer;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy::input::mouse::MouseMotion;

#[derive(Resource)]
pub struct PlayerControl {
    pub first_person: bool,
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
    player: Query<Entity, With<LocalPlayer>>,
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
    mut query: Query<(&mut Transform, &mut Velocity), With<LocalPlayer>>,
    keyboard: Res<Input<KeyCode>>,
    mut mouse: EventReader<MouseMotion>,
    mouse_button: Res<Input<MouseButton>>,
    player_control: Res<PlayerControl>,
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

    if player_control.first_person || mouse_button.pressed(MouseButton::Left) {
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