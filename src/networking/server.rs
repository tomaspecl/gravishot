use super::{NetConfig, Connection, Response, NetMarker};

use bevy::prelude::*;

use bevy_pigeon::sync::NetEntity;
use carrier_pigeon::{Server, net::Config};

use std::{net::ToSocketAddrs, sync::atomic::{AtomicU64, Ordering}};

pub static NET_ENTITY_ID_COUNTER: NetEntityIdCounter = NetEntityIdCounter(AtomicU64::new(0));
pub struct NetEntityIdCounter(AtomicU64);

impl NetEntityIdCounter {
    pub fn get_new(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst)
    }
}

pub fn handle_cons(
    mut server: ResMut<Server>,
    mut ew_sync_transform: EventWriter<bevy_pigeon::SyncC<Transform>>,
    net_entities: Query<(Entity,&NetEntity,&NetMarker)>,
    transforms: Query<&Transform>,
    players: Query<&crate::player::Player>,
    map: Res<crate::map::Map>,
) {
    let _disconnected = server.handle_disconnects(|cid, status| {
        println!("Connection {cid} disconnected with status: \"{status}\"");
    });

    let mut new_clients = Vec::new();

    let _connected = server.handle_new_cons(|cid, con: Connection| {
        println!("Connection {cid} connected with status: \"{con:?}\"");

        new_clients.push(cid);

        // Force a sync of the players so the new player has updated positions.
        ew_sync_transform.send(bevy_pigeon::SyncC::default());

        (true,Response {
            cid,
            map: map.clone(),
        })
    });

    //send list of current NetEntities (currently just clients, later bullets and grenades) to now connected clients
    for (entity,nid,marker) in net_entities.iter() {
        match marker {
            NetMarker::Player => {
                let msg = super::SpawnPlayer {
                    cid: players.get(entity).unwrap().cid,
                    nid: nid.id,
                    transform: transforms.get(entity).unwrap().clone().into(),
                };
                for &client in &new_clients {
                    server.send_to(client, &msg).unwrap();
                }
            },
            NetMarker::Bullet => todo!(),
        }
    }
}

pub fn send_player_spawns(
    server: Res<Server>,
    mut event: EventReader<crate::player::SpawnPlayerEvent>,
) {
    for event in event.iter() {
        let player: super::SpawnPlayer = event.clone().into();

        server.broadcast(&player).unwrap();
    }
}

pub fn handle_player_spawn_requests(
    server: Res<Server>,
    mut event: EventWriter<crate::player::SpawnPlayerEvent>,
) {
    event.send_batch(server.recv::<super::RequestPlayer>().map(
        |msg| {
            crate::player::SpawnPlayerEvent {
                cid: msg.cid,
                nid: NET_ENTITY_ID_COUNTER.get_new(),
                transform: Transform::from_xyz(100.0,0.0,0.0),
            }
        }
    ));
}

pub fn create_server(
    mut commands: Commands,
    config: ResMut<NetConfig>,
) {
    let listen_addr = config.ip_port.to_socket_addrs().unwrap().next().unwrap();    //TODO: when carrier_pigeon updates, can pass in the string directly
    let parts = config.msg_table.clone();
    let config = Config::default();

    println!("socket: {listen_addr}");

    let server = Server::new(listen_addr, parts, config).unwrap();
    commands.insert_resource(server);
}