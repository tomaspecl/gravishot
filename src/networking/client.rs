use super::{NetConfig, Connection, Response};

use bevy::prelude::*;
use carrier_pigeon::{Client, net::Config};

use std::net::ToSocketAddrs;

pub fn create_client(
    mut commands: Commands,
    config: ResMut<NetConfig>,
) {
    let peer = config.ip_port.to_socket_addrs().unwrap().next().unwrap();   //TODO: when carrier_pigeon updates, can pass in the string directly
    let parts = config.msg_table.clone();
    let config = Config::default();
    let con_msg = Connection {};

    println!("socket: {peer}");

    let pending_client = Client::new(peer,parts,config,con_msg);
    // For simplicity, just block until the connection is made. Realistically you would add the PendingConnection to
    //      The resources and poll it.
    let (client, response): (Client, Response) = pending_client.block().unwrap();
    info!("{:?}", response);
    commands.insert_resource(client);
    commands.insert_resource(response.map);
}