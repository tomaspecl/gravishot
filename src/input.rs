// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::networking::rollback::{ROLLBACK_ID_COUNTER, Rollback, LEN};
use crate::networking::LocalPlayer;
use crate::player::player_control::PlayerControl;
use crate::player::Player;

use bevy_gravirollback::prelude::*;

use bevy::prelude::*;

use bevy::ecs::event::EventCursor;
use bevy::utils::{HashMap, Entry};
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

#[derive(Resource, Reflect, Default, Serialize, Deserialize, Clone)]
#[reflect(Resource)]
pub struct Inputs(pub HashMap<Player, Input>);

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
pub struct LocalInput(pub Input);

/// Input from one player for one frame, also used for local player input
#[derive(Reflect, Default, Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Input {
    /// Contains data about pressed keys and mouse buttons
    #[serde(serialize_with = "serialize")]
    #[serde(deserialize_with = "deserialize")]
    pub buttons: Buttons,
    /// Contains data about mouse movements TODO: instead CameraDelta
    pub mouse: MouseDelta,
    /// Various signals that a player could send, like spawning or shooting
    pub signals: Signals,
}
impl Input {
    pub fn is_empty(&self) -> bool {
        self.buttons.is_none() && self.mouse.is_empty() && self.signals.is_empty()
    }
}

enum I {
    K(KeyCode),
    M(MouseButton)
}

type ButtonsSize = u16;
#[bitmask(u16)]
#[derive(Reflect)]
pub enum Buttons {
    W,
    S,
    A,
    D,
    Q,
    E,
    Shift,
    Space,
}
impl Default for Buttons { fn default() -> Self { Self::none() } }

pub const MOUSE_SCALE: f32 = 100.0;

#[derive(Reflect, Default, Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct MouseDelta {
    pub deltas: Vec<(i16,i16)>,  //TODO: optimize size, possibly by combining all deltas into a single delta with the same effect?
}
impl MouseDelta {
    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty()
    }
}

impl Buttons {
    pub fn set(&mut self, button: Buttons) {
        self.bits |= button.bits;
    }
    fn set_i(&mut self, button: I, _control: &PlayerControl) {
        use self::I::K;
        self.bits |= match button {
            K(KeyCode::KeyW)        => Buttons::W,
            K(KeyCode::KeyS)        => Buttons::S,
            K(KeyCode::KeyA)        => Buttons::A,
            K(KeyCode::KeyD)        => Buttons::D,
            K(KeyCode::KeyQ)        => Buttons::Q,
            K(KeyCode::KeyE)        => Buttons::E,
            K(KeyCode::ShiftLeft)   => Buttons::Shift,
            K(KeyCode::Space)       => Buttons::Space,
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

#[derive(Reflect, Default, Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Signals {
    pub shoot: Option<ShootSignal>,
    pub spawn: Option<PlayerSpawnSignal>,
}
impl Signals {
    pub fn is_empty(&self) -> bool {
        self.shoot.is_none() && self.spawn.is_none()
    }
}

#[derive(Reflect, Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct ShootSignal {
    pub id: RollbackID,
}
#[derive(Reflect, Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct PlayerSpawnSignal {
    pub body: RollbackID,
    pub gun: RollbackID,
}

pub fn get_local_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    player_control: Res<PlayerControl>,
    //mut spawn_events: EventWriter<crate::spawning::LocalSpawnEvent>,
    mut local_input: ResMut<LocalInput>,

    //for testing, playing beeps
    //mut commands: Commands,
    //mut pitch_assets: ResMut<Assets<Pitch>>,
    //client_marker: Option<Res<crate::networking::client::ClientMarker>>,
) {
    //https://bevyengine.org/examples/Audio/pitch/
    /*if client_marker.is_some() {
        commands.spawn(PitchBundle {
            source: pitch_assets.add(Pitch::new(1000.0, std::time::Duration::from_millis(100))),
            settings: PlaybackSettings::DESPAWN,
        });
    }*/

    let input = &mut local_input.0;
    for key in keyboard.get_pressed() {
        input.buttons.set_i(I::K(*key), &player_control);
    }
    for mouse_button in mouse_button.get_pressed() {
        input.buttons.set_i(I::M(*mouse_button), &player_control);
    }

    if player_control.first_person || mouse_button.pressed(MouseButton::Left) {
        for &MouseMotion{ delta: Vec2 {mut x,mut y} } in mouse_motion.read() {
            if player_control.first_person {
                x *= player_control.sensitivity;
                y *= player_control.sensitivity;
            }
            //assert!((x-x.trunc()).abs()<0.01);
            //assert!((y-y.trunc()).abs()<0.01);
            
            let x = ((MOUSE_SCALE*x).trunc() as i32).min(i16::MAX as i32).max(i16::MIN as i32) as i16;
            let y = ((MOUSE_SCALE*y).trunc() as i32).min(i16::MAX as i32).max(i16::MIN as i32) as i16;
            input.mouse.deltas.push((x,y));
        }
    }else{
        mouse_motion.clear();
    }

    //println!("testing shooting");
    if mouse_button.pressed(MouseButton::Left) && player_control.first_person || keyboard.pressed(KeyCode::KeyG) {
        //println!("shooting");
        input.signals.shoot = Some(ShootSignal {
            id: ROLLBACK_ID_COUNTER.get_new(),
        });
        //spawn_events.send(crate::spawning::LocalSpawnEvent::Bullet);
    }
}

//                                      Server -> receive Client messages -> map Input to UpdateInputEvent --v
// Client/(Server with local Client) -> LocalInputEvent -> handle_local_input_event -> emit UpdateInputEvent -> handle_update_input_event (check conditions and update Rollback<Inputs>)
//                                                                                  |                                       |--(we are the Server)--> broadcast Input
//                                                                                  --(we are a Client)--> send Input to the Server

pub fn handle_local_input_event(
    mut local_input: ResMut<LocalInput>,
    mut input_events: EventWriter<UpdateInputEvent>,
    mut client: Option<ResMut<bevy_quinnet::client::QuinnetClient>>,
    frame: Res<Frame>,
    local_player: Res<LocalPlayer>,
) {
    let local_player = local_player.0;

    let input = std::mem::take(&mut local_input.0);

    input_events.send(UpdateInputEvent {
        frame: *frame,
        player: local_player,
        input: input.clone(),
    });

    if let Some(ref mut client) = client {  //send local Input to the Server
        if !input.is_empty() {
            //println!("client sending input frame {frame}");
            client.connection_mut().try_send_message_on(1, //UnorderedReliable
                crate::networking::ClientMessage::Input(*frame, input)
            );
        }
    }
}

#[derive(Event, Serialize, Deserialize, Clone)]
pub struct UpdateInputEvent {
    pub frame: Frame,
    pub player: Player,
    pub input: Input,
}

pub fn handle_update_input_event(
    mut events: ResMut<Events<UpdateInputEvent>>,
    mut event_cursor: Local<EventCursor<UpdateInputEvent>>,
    mut inputs: ResMut<Rollback<Inputs>>,
    last_frame: Res<LastFrame>,
    frames: Res<Rollback<Frame>>,
    mut modified: ResMut<Rollback<Modified>>,
    mut server: Option<ResMut<bevy_quinnet::server::QuinnetServer>>,
) {
    let mut events_to_resend = Vec::new();

    for event in event_cursor.read(&events) {
        let UpdateInputEvent { frame, player, input } = event.clone();
        //println!("handling input event {frame:?}");
        if input.is_empty() {
            continue;
        }

        //println!("update input event {frame:?} player {player:?} {input:?}");
        let update = frame.0 < last_frame.0;
        if frame.0 > last_frame.0 {
            warn!("future update event {frame:?} {last_frame:?} player {player:?}, saving for next frame");
            events_to_resend.push(event.clone());
            continue;
        }

        let index = index::<LEN>(frame.0);
        if frames[index].0 == frame.0 {
            //insert this input
            match inputs.0[index].0.entry(player) {
                Entry::Vacant(entry) => {
                    if !input.is_empty() {
                        //println!("input of player {player:?} from {frame:?} got inserted {input:?}");
                    }
                    if let Some(ref mut server) = server {
                        let endpoint = server.endpoint_mut();
                        let mut clients = endpoint.clients();
                        clients.retain(|&x| x!=player.0);   //send to everyone except the Client that sent it
                        endpoint.try_send_group_message_on(
                            clients.iter(),
                            1,  //UnorderedReliable
                            crate::networking::ServerMessage::Input(event.clone()),
                        );
                    }
                    entry.insert(input);
                    modified[index].0 |= update;
                },
                Entry::Occupied(mut entry) => {
                    if server.is_none() {
                        if *entry.get() != input {
                            warn!("input of player {player:?} from {frame:?} got changed");
                            entry.insert(input);
                            modified[index].0 |= update;
                        }
                    }else{
                        warn!("input of player {player:?} from {frame:?} tried to change");
                    }
                },
            }
        }else{
            warn!("too old frame updated {frame:?} stored {:?} last {last_frame:?} player {player:?}", frames[index]);
        }
    }

    for event in events_to_resend {
        events.send(event);
    }
}