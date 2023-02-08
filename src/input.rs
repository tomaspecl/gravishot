use crate::networking::LocalPlayer;
use crate::networking::rollback::Inputs;
use crate::player::player_control::PlayerControl;

use bevy::prelude::*;

use bevy::input::mouse::MouseMotion;
use bitmask_enum::bitmask;
use serde::{Serialize, Deserialize};

fn serialize<S: serde::Serializer>(data: &Buttons, serializer: S) -> Result<S::Ok, S::Error> {
    data.bits().serialize(serializer)
}
fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Buttons, D::Error> {
    Ok(Buttons {
        bits: ButtonsSize::deserialize(deserializer)?,
    })
}

/// Input from one player for one frame
#[derive(Reflect, FromReflect, Default, Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Input {
    /// Contains data about pressed keys and mouse buttons
    #[serde(serialize_with = "serialize")]
    #[serde(deserialize_with = "deserialize")]
    pub buttons: Buttons,
    /// Contains data about mouse movements TODO: instead CameraDelta
    pub mouse: MouseDelta,
}

enum I {
    K(KeyCode),
    M(MouseButton)
}

type ButtonsSize = u16;
#[bitmask(u16)]
#[derive(Reflect, FromReflect)]
pub enum Buttons {
    W,
    S,
    A,
    D,
    Q,
    E,
    Shift,
    Space,
    Shoot,
    Spawn,
}
impl Default for Buttons { fn default() -> Self { Self::none() } }

pub const MOUSE_SCALE: f32 = 100.0;

#[derive(Reflect, FromReflect, Default, Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct MouseDelta {
    pub deltas: Vec<(i16,i16)>,  //TODO: optimize size, possibly by combining all deltas into a single delta with the same effect?
}

impl Buttons {
    pub fn set(&mut self, button: Buttons) {
        self.bits |= button.bits;
    }
    fn set_i(&mut self, button: I, control: &PlayerControl) {
        use self::I::{K, M};
        use MouseButton::Left;
        use KeyCode::*;
        self.bits |= match button {
            M(Left) if control.first_person => Buttons::Shoot,
            K(W)        => Buttons::W,
            K(S)        => Buttons::S,
            K(A)        => Buttons::A,
            K(D)        => Buttons::D,
            K(Q)        => Buttons::Q,
            K(E)        => Buttons::E,
            K(LShift)   => Buttons::Shift,
            K(Space)    => Buttons::Space,
            //K()        => Buttons::,
            _ => Buttons::none()
        }.bits;
    }
}

pub use crate::pressed;
#[macro_export]
macro_rules! pressed {
    (($buttons:ident); $($button:expr => $code:stmt;)+) => {
        $(if $buttons.contains($button) { $code });+
    }
}

pub fn clear(mut inputs: ResMut<Inputs>) {
    inputs.0.clear();
}

pub fn get_local_input(
    keyboard: Res<bevy::input::Input<KeyCode>>,
    mouse_button: Res<bevy::input::Input<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut inputs: ResMut<Inputs>,
    local_player: Res<LocalPlayer>,
    player_control: Res<PlayerControl>,
) {
    let input = inputs.0.entry(local_player.0).or_default();
    let buttons = &mut input.buttons;
    let mouse = &mut input.mouse;

    for key in keyboard.get_pressed() {
        buttons.set_i(I::K(*key), &player_control);
    }
    for mouse_button in mouse_button.get_pressed() {
        buttons.set_i(I::M(*mouse_button), &player_control);
    }
    if player_control.first_person || mouse_button.pressed(MouseButton::Left) {
        for &MouseMotion{ delta: Vec2 {mut x,mut y} } in mouse_motion.iter() {
            if player_control.first_person {
                x *= player_control.sensitivity;
                y *= player_control.sensitivity;
            }
            //assert!((x-x.trunc()).abs()<0.01);
            //assert!((y-y.trunc()).abs()<0.01);
            
            let x = ((MOUSE_SCALE*x).trunc() as i32).min(i16::MAX as i32).max(i16::MIN as i32) as i16;
            let y = ((MOUSE_SCALE*y).trunc() as i32).min(i16::MAX as i32).max(i16::MIN as i32) as i16;
            mouse.deltas.push((x,y));
        }
    }
}