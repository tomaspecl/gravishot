use super::{NetConfig, Connection, Response};

use bevy::prelude::*;
use carrier_pigeon::{Client, net::Config};

use std::net::ToSocketAddrs;

pub fn receive_player_spawns(
    client: ResMut<Client>,
    mut event: EventWriter<crate::player::SpawnPlayerEvent>,
) {
    event.send_batch(client.recv::<super::SpawnPlayer>().map(|m| m.m.clone().into()));
}

pub fn receive_player_despawns(
    client: ResMut<Client>,
    mut event: EventWriter<crate::player::DespawnPlayerEvent>,
) {
    event.send_batch(client.recv::<super::DespawnPlayer>().map(|m| crate::player::DespawnPlayerEvent(m.m.0)));
}

pub fn create_client(
    mut commands: Commands,
    mut myconfig: ResMut<NetConfig>,
) {
    let peer = myconfig.ip_port.to_socket_addrs().unwrap().next().unwrap();   //TODO: when carrier_pigeon updates, can pass in the string directly
    let parts = myconfig.msg_table.clone();
    let config = Config::default();
    let con_msg = Connection {};

    println!("socket: {peer}");

    let pending_client = Client::new(peer,parts,config,con_msg);
    // For simplicity, just block until the connection is made. Realistically you would add the PendingConnection to
    //      The resources and poll it.
    let (client, response): (Client, Response) = pending_client.block().unwrap();
    info!("{:?}", response);
    myconfig.local_player_cid = response.cid;
    commands.insert_resource(client);
    commands.insert_resource(response.map);
}