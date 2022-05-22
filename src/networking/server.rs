use super::{NetConfig, Connection, Response, Disconnect};
use crate::gamestate::GameState;

//use crate::patch::system_set::SystemSet;

use bevy::prelude::*;
use carrier_pigeon::{Server, net::Config};

use std::net::ToSocketAddrs;

pub fn register_systems(app: &mut App) {
    app
    .add_system_set(
        SystemSet::on_enter(GameState::Server)
            .with_system(create_server)
            .with_system(crate::map::generate_map)
    )
    .add_system_set(
        SystemSet::on_in_stack_update(GameState::Server)    //TODO: does not work, only updates when no other state on the stack
            .with_system(bevy_pigeon::app::server_tick.label(bevy_pigeon::NetLabel))
            .with_system(handle_cons.after(bevy_pigeon::NetLabel))
            //.with_system(|| println!("server2"))
    );
}

fn handle_cons(
    server: Option<ResMut<Server>>,
    mut ew_sync_transform: EventWriter<bevy_pigeon::SyncC<Transform>>,
) {
    println!("server"); //debug print to know when this system runs
    if let Some(mut server) = server {
        server.handle_disconnects(|cid, status| {
            println!("Connection {cid} disconnected with status: \"{status}\"");
        });

        server.handle_new_cons(|cid, con: Connection| {
            println!("Connection {cid} connected with status: \"{con:?}\"");

            // Force a sync of the players so the new player has updated positions.
            ew_sync_transform.send(bevy_pigeon::SyncC::default());

            (true,Response {})
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