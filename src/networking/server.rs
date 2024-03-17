// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use super::rollback::*;
use super::rollback::{State, States, Snapshot};
use super::{ClientMessage, ServerMessage, NetConfig};
use crate::input::{UpdateInputEvent, Inputs};

use bevy_gravirollback::new::*;

use bevy::prelude::*;

use bevy::utils::{HashMap, Entry};
use bevy_quinnet::server::{Server, ServerConfiguration, certificate::CertificateRetrievalMode, ConnectionLostEvent, ConnectionEvent};

use std::net::ToSocketAddrs;

#[derive(Resource)]
pub struct ServerMarker;

pub struct SummaryTimer(Timer);
impl Default for SummaryTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

pub fn handle(
    mut server: ResMut<Server>,
    //local_player: Option<Res<super::LocalPlayer>>,  //TODO: can this fail?
    mut players: ResMut<super::PlayerMap>,
    
    mut commands: Commands,

    snapshot_info: Res<SnapshotInfo>,
    //mut reader: Local<bevy::ecs::event::ManualEventReader<UpdateInputEvent<(Player, Input)>>>,
    mut input_event: EventWriter<UpdateInputEvent>,
    mut state_event: EventWriter<UpdateStateEvent<State>>,

    map: Res<crate::map::Map>,
    update_timer: Res<crate::gamestate::UpdateTimer>,
    
    mut events_conn: EventReader<ConnectionEvent>,
    mut events_lost: EventReader<ConnectionLostEvent>,
) {
    let now = snapshot_info.last;
    let endpoint = server.endpoint_mut();

    for event in events_conn.read() {
        println!("ConnectionEvent: Player {} connected",event.id);
    }

    //handle lost connections
    for event in events_lost.read() {
        let player = crate::player::Player(event.id);
        println!("Player {} disconnected",player.0);
        players.0.remove(&player);
        endpoint.try_broadcast_message(ServerMessage::Disconnected(player));
        commands.add(crate::player::despawn_player(player));
    }

    //send local player input
    /*if let Some(player) = local_player {
        let player = player.0;
        for UpdateInputEvent { frame, input } in reader.read(&mut input_event) {
            if player==input.0 {
                let input = input.1.clone();
                if !input.buttons.is_none() || !input.mouse.deltas.is_empty() {
                    endpoint.try_broadcast_message_on(
                        ChannelId::UnorderedReliable,
                        ServerMessage::Input(*frame, player, input)
                    );
                }
            }else{
                println!("local player {player:?} input player {:?}",input.0);
            }
        }
    }*/

    //handle received messages
    for client_id in endpoint.clients() {
        let player = crate::player::Player(client_id);
        while let Some(msg) = endpoint.try_receive_message_from::<ClientMessage>(client_id) {
            match msg {
                ClientMessage::Connect => {
                    let rollback = ROLLBACK_ID_COUNTER.get_new();
                    println!("Player {} connected with rollback {}",player.0,rollback.0);
                    if let Entry::Vacant(e) = players.0.entry(player) {
                        e.insert(rollback);
                    }else{
                        warn!("player already exists!")
                    }
                    endpoint.try_send_message(client_id, ServerMessage::ConnectionGranted(
                        player,
                        map.clone(),
                        States {
                            last_frame: now,
                            frame_0_time: update_timer.frame_0_time,
                        },
                    ));
                    endpoint.try_broadcast_message(ServerMessage::Connected(player, rollback));
                },
                ClientMessage::Input(frame, input) => {
                    println!("received input frame {frame}");

                    input_event.send(UpdateInputEvent {
                        frame,
                        player,
                        input,
                    });

                    /*
                    match SnapshotRef::new(now, frame, &mut snapshots, &mut inputs) {
                        SnapshotType::Past(snapshot) | SnapshotType::Now(snapshot) => {
                            match snapshot.inputs.0.entry(player) {
                                Entry::Occupied(e) => {
                                    //when the server already has the players input it will
                                    //reject it and send correction back to the client
                                    print!("occupied");
                                    if *e.get()!=input {
                                        print!(" different");
                                        endpoint.try_send_message_on(
                                            client_id,
                                            ChannelId::UnorderedReliable,
                                            ServerMessage::Input(frame, player, e.get().clone())
                                        );
                                    }
                                    println!();
                                },
                                Entry::Vacant(e) => {
                                    println!("vacant");
                                    e.insert(input.clone());
                                    *snapshot.modified = true;
                                    let mut clients = endpoint.clients();
                                    clients.retain(|&x| x!=player.0);
                                    endpoint.try_send_group_message_on(
                                        clients.iter(),
                                        ChannelId::UnorderedReliable,
                                        ServerMessage::Input(frame, player, input)
                                    );
                                },
                            }
                        },
                        SnapshotType::Future { now: _ } => {
                            println!("received Input from the future! Asking to slow down. now {now} frame {frame}");
                            endpoint.try_send_message(
                                client_id,
                                ServerMessage::SlowDown(now)
                            );
                        },
                        SnapshotType::SuperPast => ()
                    }
                    */
                },
                ClientMessage::Correction(frame, state) => {
                    //TODO: we should have some policy for rejecting too big changes
                    if let Some(&id) = players.0.get(&player) {
                        state_event.send(UpdateStateEvent {frame, id, state});
                    }

                    /*
                    match SnapshotRef::new(now, frame, &mut snapshots, &mut inputs) {
                        SnapshotType::Past(snapshot) | SnapshotType::Now(snapshot) => {
                            if let Some(rollback) = players.0.get(&player) {
                                if let Some(state) = snapshot.states.get_mut(rollback) {
                                    let s = state.state;
                                    let c = state_cor.state;
                                    if !state.fixed && !state_cor.fixed && s.entity==c.entity {
                                        let condition = 
                                            //TODO: move magic values into constants
                                            s.transform.translation.distance(c.transform.translation) < 0.1
                                            && s.transform.rotation.angle_between(c.transform.rotation) < 0.1
                                            && s.transform.scale == c.transform.scale
                                            && s.velocity.linvel.distance(c.velocity.linvel) < 0.1  
                                            && s.velocity.angvel.distance(c.velocity.angvel) < 0.1;
                                        if condition {
                                            *state = state_cor;
                                            *snapshot.modified = true;  //TODO: resend to others
                                        }else{
                                            //send correction
                                            state.fixed = true;
                                            endpoint.try_send_message(
                                                client_id,
                                                ServerMessage::StateSummary(frame, snapshot.clone(), players.clone())
                                            );
                                        }
                                        continue    //skip the disconnection
                                    }
                                }
                            }
                            //disconnect the player - player is probably cheating
                            let _ = endpoint.disconnect_client(client_id);
                        },
                        SnapshotType::Future { now: _ } => {
                            println!("received State from the future! Asking to slow down. now {now} frame {frame}");
                            endpoint.try_send_message(
                                client_id,
                                ServerMessage::SlowDown(now)
                            );
                        },
                        SnapshotType::SuperPast => (),
                    }
                    */
                }
            }
        }
    }
}

pub fn send_state_summary(
    server: Res<Server>,
    query: Query<(&RollbackID, &Rollback<PhysicsBundle>, &super::EntityType)>,
    inputs: Res<Rollback<Inputs>>,
    snapshot_info: Res<SnapshotInfo>,
    players: Res<super::PlayerMap>,
    time: Res<Time>,
    mut timer: Local<SummaryTimer>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        println!("sending summary");
        let endpoint = server.endpoint();
        //do not send the latest snapshot, instead send old, that way it is likely not going to change anymore
        let offset = SNAPSHOTS_LEN as u64 / 4;
        let frame = if snapshot_info.last > offset {
            snapshot_info.last - offset
        }else{return};
        
        let index = snapshot_info.index(frame);

        let mut states = HashMap::new();
        for (&id, physics_bundle, &entity_type) in &query {
            states.insert(id, State(physics_bundle.0[index].clone(), entity_type));
        }

        let snapshot = Snapshot {
            states,
            inputs: inputs.0[index].clone(),
        };
        endpoint.try_broadcast_message(ServerMessage::StateSummary(frame, snapshot, players.clone()))
    }
}

pub fn start(
    mut server: ResMut<Server>,
    config: Res<NetConfig>,
) {
    let addr = config.ip_port.to_socket_addrs().unwrap().next().unwrap();
    println!("socket: {addr}");
    server.start_endpoint(
        ServerConfiguration::from_addr(addr),
        CertificateRetrievalMode::GenerateSelfSigned {
            server_hostname: "GraviShot server".to_string(),    //TODO: allow manually setting this
        },
    ).unwrap();
}