use super::rollback::{SnapshotRef, SnapshotType, Snapshots, Inputs, Rollback, MyState};
use super::{ClientMessage, ServerMessage, NetConfig};

use bevy::prelude::*;

use bevy::utils::Entry;
use bevy_quinnet::{server::{Server, ServerConfigurationData, certificate::CertificateRetrievalMode, ConnectionLostEvent, ConnectionEvent}, shared::channel::ChannelId};

use std::{net::ToSocketAddrs, sync::atomic::{AtomicU64, Ordering}};

pub static ROLLBACK_ID_COUNTER: RollbackIdCounter = RollbackIdCounter(AtomicU64::new(0));
pub struct RollbackIdCounter(AtomicU64);

impl RollbackIdCounter {
    pub fn get_new(&self) -> Rollback {
        Rollback(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

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
    local_player: Option<Res<super::LocalPlayer>>,  //TODO: can this fail?
    mut players: ResMut<super::PlayerMap>,
    
    mut commands: Commands,

    mut snapshots: ResMut<Snapshots<MyState>>,
    mut inputs: ResMut<Inputs>,

    map: Res<crate::map::Map>,
    
    mut events_conn: EventReader<ConnectionEvent>,
    mut events_lost: EventReader<ConnectionLostEvent>,

    time: Res<Time>,
    mut timer: Local<SummaryTimer>,
) {
    let now = snapshots.frame;
    let endpoint = server.endpoint_mut();

    for event in events_conn.iter() {
        println!("ConnectionEvent: Player {} connected",event.id);
    }

    //handle lost connections
    for event in events_lost.iter() {
        let player = crate::player::Player(event.id);
        println!("Player {} disconnected",player.0);
        players.0.remove(&player);
        endpoint.try_broadcast_message(ServerMessage::Disconnected(player));
        commands.add(crate::player::despawn_player(player));
    }

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
                        snapshots.clone(),
                    ));
                    endpoint.try_broadcast_message(ServerMessage::Connected(player, rollback));
                },
                ClientMessage::RequestPlayer => {
                    inputs.0.entry(player).or_default().buttons.set(crate::input::Buttons::Spawn);
                    //TODO: broadcast SpawnPlayer
                },
                ClientMessage::Input(frame, input) => {
                    //println!("received input frame {frame}: {}",input.buttons.bits());

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
                },
                ClientMessage::Correction(frame, state_cor) => {
                    match SnapshotRef::new(now, frame, &mut snapshots, &mut inputs) {
                        SnapshotType::Past(snapshot) | SnapshotType::Now(snapshot) => {
                            if let Some(rollback) = players.0.get(&player) {
                                if let Some(state) = snapshot.states.get_mut(rollback) {
                                    let s = state.state;
                                    let c = state_cor.state;
                                    if !state.fixed && !state_cor.fixed && s.entity==c.entity {
                                        let condition = 
                                            //TODO: move magic values into constants
                                            s.transform.translation.abs_diff_eq(c.transform.translation, 0.1)
                                            && s.transform.rotation.abs_diff_eq(c.transform.rotation, 0.1)
                                            && s.transform.scale == c.transform.scale
                                            && s.velocity.linvel.abs_diff_eq(c.velocity.linvel, 0.1)  
                                            && s.velocity.angvel.abs_diff_eq(c.velocity.angvel, 0.1);
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
                }
            }
        }
    }

    if timer.0.tick(time.delta()).just_finished() {
        //send state summary
        println!("sending summary");
        let snapshot = snapshots.buffer.back().expect("should contain at least one Snapshot").clone();
        endpoint.try_broadcast_message(ServerMessage::StateSummary(now, snapshot, players.clone()))
    }

    //send local player input
    let Some(player) = local_player else{return};
    let player = player.0;
    let Some(input) = inputs.0.get(&player) else{return};
    endpoint.try_broadcast_message_on(
        ChannelId::UnorderedReliable,
        ServerMessage::Input(now, player, input.clone())
    );
}

pub fn start(
    mut server: ResMut<Server>,
    config: ResMut<NetConfig>,
) {
    let addr = config.ip_port.to_socket_addrs().unwrap().next().unwrap();
    println!("socket: {addr}");
    server.start_endpoint(
        ServerConfigurationData::new(
            addr.ip().to_string(),
            addr.port(),
            "0.0.0.0".to_string()
        ),
        CertificateRetrievalMode::GenerateSelfSigned,
    ).unwrap();
}