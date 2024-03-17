// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::input::UpdateInputEvent;
use super::rollback::*;
use super::rollback::State;
use super::{ClientMessage, ServerMessage};

use bevy_gravirollback::new::*;

use bevy::prelude::*;

use bevy::utils::Entry;
use bevy_quinnet::client::{Client, connection::ConnectionConfiguration, certificate::CertificateVerificationMode};

use std::net::ToSocketAddrs;

#[derive(Resource)]
pub struct ClientMarker;

pub fn handle(
    mut client: ResMut<Client>,
    //local_player: Option<Res<super::LocalPlayer>>,      //TODO: can this fail?
    mut players: ResMut<super::PlayerMap>,
    //mut net_config: ResMut<super::NetConfig>,
    mut commands: Commands,
    mut state: ResMut<NextState<crate::gamestate::GameState>>,

    mut update_timer: ResMut<crate::gamestate::UpdateTimer>,

    mut snapshot_info: ResMut<SnapshotInfo>,
    mut input_event: EventWriter<UpdateInputEvent>,
    mut state_event_writer: EventWriter<UpdateStateEvent<State>>,
    mut rollback_map: ResMut<RollbackMap>,
) {
    while let Some(msg) = client.connection_mut().try_receive_message::<ServerMessage>() {
        match msg {
            //TODO: move ConnectionGranted in different GameState
            ServerMessage::ConnectionGranted(player, map, states) => {
                //TODO: move somewhere else (system set when ClientSetup) such that this system does not need ResMut<NetConfig>?

                ROLLBACK_ID_COUNTER.0.store((player.0 << 32).into(), std::sync::atomic::Ordering::SeqCst);

                commands.insert_resource(super::LocalPlayer(player));
                commands.insert_resource(map);
                
                //TODO: sync our frame number with the server
                let last = states.last_frame;
                let frame_0_time = states.frame_0_time;
                update_timer.frame_0_time = frame_0_time;

                snapshot_info.last = last;
                snapshot_info.current = last;
                let index = snapshot_info.index(last);
                snapshot_info.snapshots[index].frame = last;
                println!("client connected player {player:?} last frame {}", snapshot_info.last);

                state.set(crate::gamestate::GameState::Running);
            },
            ServerMessage::Connected(player, rollback) => {
                println!("Player {} connected with rollback {}",player.0,rollback.0);
                if let Entry::Vacant(e) = players.0.entry(player) {
                    e.insert(rollback);
                }else{
                    panic!("player already exists!")
                }
            },
            ServerMessage::Disconnected(player) => {
                println!("Player {} disconneted",player.0);
                players.0.remove(&player);
                commands.add(crate::player::despawn_player(player));
            },
            /*ServerMessage::SpawnPlayer { player, rollback, transform } => {
                event_spawn.send(crate::player::SpawnPlayerEvent {
                    player,
                    rollback,
                    transform,
                });
            }
            ServerMessage::DespawnPlayer(player) => {
                event_despawn.send(crate::player::DespawnPlayerEvent(player));
            },*/
            ServerMessage::Input(update_input_event) => {
                //println!("server message input frame {frame} player {player:?}");
                input_event.send(update_input_event);
            },
            ServerMessage::SlowDown(_frame) => {
                todo!()
                /*if now>frame {
                    //past snapshot
                    commands.insert_resource(super::rollback::FuturePastSnapshot::<MyState> {
                        frame,
                        snapshot: Snapshot::default()
                    });
                }*/
            }
            ServerMessage::StateSummary(frame, snapshot_summary, players_summary) => {
                let current = snapshot_info.current;
                let diff = current as i64 - frame as i64;
                println!("got summary frame {frame} current {current} diff {diff}");
                *players = players_summary; //TODO: this might not be enough

                let inputs = snapshot_summary.inputs.0;
                let states = snapshot_summary.states;

                let mut deleted = Vec::new();
                for (id, &entity) in &rollback_map.0 {
                    if !states.contains_key(id) {
                        //println!("got summary despawning {entity:?}");
                        commands.entity(entity).despawn_recursive();
                        deleted.push(entity);
                    }
                }
                for e in deleted {
                    rollback_map.remove(e);
                }

                input_event.send_batch(inputs.into_iter().map(|(player, input)| UpdateInputEvent { frame, player, input }));
                state_event_writer.send_batch(states.into_iter().map(|(id, state)| UpdateStateEvent {frame, id, state}));

                /*
                //TODO: move this into update event handler
                let summary_states = &mut snapshot_summary.states;
                let summary_inputs = &mut snapshot_summary.inputs.0;
                let mut state_cor = None;
                let mut input_cor = None;
                let mut should_return = false;

                fn add_local_player_input(
                    my_inputs: &mut HashMap<Player, Input>,
                    local_player: Player,
                    summary_inputs: &mut HashMap<Player, Input>,
                    input_cor: &mut Option<Input>)
                {
                    if !summary_inputs.contains_key(&local_player) {
                        if let Some(my_input) = my_inputs.get(&local_player) {
                            *input_cor = Some(my_input.clone());
                            //make sure that local player input doesnt get deleted during next step
                            summary_inputs.insert(local_player, my_input.clone());
                        }
                    }
                }
                match SnapshotRef::new(last_frame, frame, &mut snapshots, &mut inputs) {
                    SnapshotType::Past(my_snapshot) | SnapshotType::Now(my_snapshot) => {
                        let my_states = my_snapshot.states;
                        let my_inputs = &mut my_snapshot.inputs.0;

                        //NOTE: sometimes the Client Player got removed from the world
                        // probably because it got deleted from states
                        // -> TODO: Client sends RequestPlayer message and then Server sends back state

                        //TODO: how does Server send that local_player is not there?
                        // perhaps we should always delete when entity is absent

                        //prepare correction of local player data for the Server
                        if let Some(local_player) = local_player {
                            add_local_player_input(my_inputs, local_player, summary_inputs, &mut input_cor);
                            if let Some(&local_player_roll) = players.0.get(&local_player) {
                                if let Some(my_player_state) = my_states.get_mut(&local_player_roll) {
                                    match summary_states.entry(local_player_roll) {
                                        Entry::Occupied(mut entry) => {
                                            let player_state = entry.get_mut ();
                                            if player_state.fixed {
                                                *my_player_state = *player_state;
                                            }else{
                                                if *player_state!=*my_player_state {
                                                    assert!(!my_player_state.fixed);
                                                    state_cor = Some(*my_player_state);
                                                    //make sure that local_player doesnt get overwritten during next step
                                                    *player_state = *my_player_state;
                                                }
                                            }
                                        },
                                        //let the player get deleted
                                        //Entry::Vacant(e) => {
                                        //    correction.states.insert(r, my_state);
                                        //    //make sure that local_player doesnt get deleted during next step
                                        //    e.insert(my_state);
                                        //},
                                        _ => ()
                                    }
                                }
                            }
                        }

                        /*let mut for_delete = Vec::new();
                        for (&roll,my_s) in my_states.iter_mut() {
                            if let Some(s) = summary_states.remove(&roll) {
                                //update local states
                                *my_s = s;
                            }else{
                                //mark for deletion local states which should not remain
                                for_delete.push(roll);
                            }
                        }
                        //delete marked states
                        for roll in for_delete {
                            my_states.remove(&roll);
                        }
                        //insert remaining states
                        for (roll,s) in summary_states.drain() {
                            my_states.insert(roll, s);
                        }*/
                        //TODO: replace with
                        *my_states = snapshot_summary.states;

                        /*let mut for_delete = Vec::new();
                        for (&player,my_in) in my_inputs.iter_mut() {
                            if let Some(i) = summary_inputs.remove(&player) {
                                //update local inputs
                                *my_in = i;
                            }else{
                                //mark for deletion local inputs which should not remain    //TODO: this should not happen
                                for_delete.push(player);
                            }
                        }
                        //delete marked inputs
                        for player in for_delete {
                            my_inputs.remove(&player);
                        }
                        //insert remaining inputs
                        for (player,i) in summary_inputs.drain() {
                            my_inputs.insert(player, i);
                        }*/
                        //TODO: replace with
                        *my_inputs = snapshot_summary.inputs.0;

                        *my_snapshot.modified = true;   //TODO: only when updated
                    },
                    SnapshotType::Future { now } => {
                        let my_inputs = &mut now.inputs.0;

                        if let Some(local_player) = local_player {
                            add_local_player_input(my_inputs, local_player, summary_inputs, &mut input_cor);
                        }

                        snapshot_summary.modified = true; //TODO: is this correct?

                        commands.insert_resource(super::rollback::FuturePastSnapshot {
                            snapshot: snapshot_summary,
                            frame,
                        });
                        should_return = true;
                    },
                    SnapshotType::SuperPast => ()
                }
                //send corrections
                if let Some(input) = input_cor {
                    client.connection().try_send_message(ClientMessage::Input(frame, input));
                }
                if let Some(state) = state_cor {
                    client.connection().try_send_message(ClientMessage::Correction(frame, state));
                }

                if should_return {return}
                */
            },
        }
    }
}

pub fn connect(mut client: ResMut<Client>, myconfig: Res<super::NetConfig>) {
    let addr = myconfig.ip_port.to_socket_addrs().unwrap().next().unwrap();

    println!("socket: {addr}");

    client.open_connection(
        ConnectionConfiguration::from_addrs(addr,str::parse("0.0.0.0:0").unwrap()),
        CertificateVerificationMode::SkipVerification,
    ).unwrap();
}

pub fn on_connect(
    mut events: EventReader<bevy_quinnet::client::connection::ConnectionEvent>,
    client: Res<Client>
) {
    if let Some(connection) = events.read().next() {
        let client_id = connection.id;   //TODO: is this really client_id?

        println!("Joining with client_id {client_id}");

        client.connection().try_send_message(ClientMessage::Connect);
    }
    events.clear();
}
