use super::{NetConfig, Connection, Response};

//use crate::patch::system_set::SystemSet;

use bevy::prelude::*;
use carrier_pigeon::{Server, net::Config};

use std::net::ToSocketAddrs;

pub fn handle_cons(
    server: Option<ResMut<Server>>,
    mut ew_sync_transform: EventWriter<bevy_pigeon::SyncC<Transform>>,
    map: Res<crate::map::Map>,
) {
    //println!("server"); //debug print to know when this system runs
    if let Some(mut server) = server {
        server.handle_disconnects(|cid, status| {
            println!("Connection {cid} disconnected with status: \"{status}\"");
        });

        server.handle_new_cons(|cid, con: Connection| {
            println!("Connection {cid} connected with status: \"{con:?}\"");

            // Force a sync of the players so the new player has updated positions.
            ew_sync_transform.send(bevy_pigeon::SyncC::default());

            (true,Response {
                map: map.clone(),
            })
        });
    }
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