// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use super::rollback::*;
use super::rollback::{State, States, Snapshot, Rollback, LEN};
use super::{ClientMessage, ServerMessage, NetConfig};
use crate::input::{UpdateInputEvent, Inputs};

use bevy_gravirollback::prelude::*;

use bevy::prelude::*;

use bevy::utils::HashMap;
use bevy_quinnet::server::{QuinnetServer, ConnectionLostEvent, ConnectionEvent};

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
    mut server: ResMut<QuinnetServer>,
    //local_player: Option<Res<super::LocalPlayer>>,  //TODO: can this fail?
    players: Query<(&crate::player::Player, &RollbackID)>,
    
    mut commands: Commands,

    last_frame: Res<LastFrame>,
    //mut reader: Local<bevy::ecs::event::ManualEventReader<UpdateInputEvent<(Player, Input)>>>,
    mut input_event: EventWriter<UpdateInputEvent>,
    mut state_event: EventWriter<UpdateStateEvent<State>>,

    map: Res<crate::map::Map>,
    update_timer: Res<crate::gamestate::UpdateTimer>,
    
    mut events_conn: EventReader<ConnectionEvent>,
    mut events_lost: EventReader<ConnectionLostEvent>,
) {
    let endpoint = server.endpoint_mut();

    for event in events_conn.read() {
        println!("ConnectionEvent: Player {} connected",event.id);
    }

    //handle lost connections
    for event in events_lost.read() {
        let player = crate::player::Player(event.id);
        println!("Player {} disconnected",player.0);
        endpoint.try_broadcast_message(ServerMessage::Disconnected(player));
        commands.queue(crate::player::despawn_player(player));
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

    if map.is_changed() {
        endpoint.try_broadcast_message(ServerMessage::MapUpdate(map.clone()));
    }

    //handle received messages
    for client_id in endpoint.clients() {
        let player = crate::player::Player(client_id);
        while let Some((_channel_id, msg)) = endpoint.try_receive_message_from::<ClientMessage>(client_id) {
            match msg {
                ClientMessage::Connect => {
                    println!("Player {player:?} connected");
                    endpoint.try_send_message(client_id, ServerMessage::ConnectionGranted(
                        player,
                        map.clone(),
                        States {
                            last_frame: *last_frame,
                            frame_0_time: update_timer.frame_0_time,
                        },
                    ));
                    endpoint.try_broadcast_message(ServerMessage::Connected(player));
                },
                ClientMessage::Input(frame, input) => {
                    println!("received input {frame:?}");

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

                    if let Some((_,&id)) = players.iter().find(|(p,_)| **p==player) {
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
    mut server: ResMut<QuinnetServer>,
    query: Query<(&RollbackID, &Rollback<Exists>, &Rollback<PhysicsBundle>, Option<(&Rollback<crate::player::HeadData>, &Rollback<crate::player::Health>)>, Option<&crate::player::Player>, &super::EntityType)>,
    inputs: Res<Rollback<Inputs>>,
    last_frame: Res<LastFrame>,
    time: Res<Time>,
    mut timer: Local<SummaryTimer>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        //println!("sending summary");

        //do not send the latest snapshot, instead send old, that way it is likely not going to change anymore
        let offset = LEN as u64 / 4;
        let frame = if last_frame.0 >= offset {
            last_frame.0 - offset
        }else{return};
        
        let index = index::<LEN>(frame);

        let mut states = HashMap::new();
        for (&id, exists, physics_bundle, player_data, player, &entity_type) in &query {
            //TODO: this also sends entities present in current frame that were not yet spawned in the past frame
            //this will then cause a crash when the Client receives it, it will spawn the entity before it should be spawned
            //and then later it will receive the spawn signal that will try to spawn the entity second time -> crash

            //this should fix it:
            let exists = exists.0[index];

            let player_data = player_data.map(|x| (x.0.0[index].clone(),x.1.0[index].clone()));
            let player = player.map(|x| x.clone());
            states.insert(id, State(physics_bundle.0[index].clone(), player_data, player, entity_type, exists));
        }

        let snapshot = Snapshot {
            states,
            inputs: inputs.0[index].clone(),
        };
        server.endpoint_mut().try_broadcast_message(ServerMessage::StateSummary(Frame(frame), snapshot))
    }
}

pub fn start(
    mut server: ResMut<QuinnetServer>,
    config: Res<NetConfig>,
) {
    let addr = config.ip_port.to_socket_addrs().unwrap().next().unwrap();

    println!("socket: {addr}");
    
    use bevy_quinnet::shared::channels::{ChannelType, ChannelsConfiguration};
    server.start_endpoint(
        bevy_quinnet::server::ServerEndpointConfiguration::from_addr(addr),
        bevy_quinnet::server::certificate::CertificateRetrievalMode::GenerateSelfSigned {
            server_hostname: "GraviShot server".to_string(),    //TODO: allow manually setting this
        },
        ChannelsConfiguration::from_types(vec![ChannelType::OrderedReliable, ChannelType::UnorderedReliable]).unwrap()
    ).unwrap();
}