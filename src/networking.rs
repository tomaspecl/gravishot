pub mod server;
pub mod client;

use bevy::prelude::*;
use bevy_pigeon::{types::NetTransform, AppExt};
use carrier_pigeon::{MsgTable, Transport, MsgTableParts};
use serde::{Serialize, Deserialize};

//when in client mode -> GameState::Running + Client resource (syncs from server) + display game + handle input
//when in server mode -> GameState::Running + Server resource (syncs to clients)
//      when connect localy -> + display game + handle input, no Client resource because it doesnt sync from server (we are the server)

pub struct NetConfig {
    pub ip_port: String,
    pub msg_table: MsgTableParts,
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        let config = NetConfig {
            ip_port: "localhost:12345".to_string(),
            msg_table: make_msg_table(app),
        };

        app.insert_resource(config);
    }
}

fn make_msg_table(app: &mut App) -> MsgTableParts {
    let mut table = MsgTable::new();

    //table.register(transport);

    app.sync_comp::<Transform, NetTransform>(&mut table, Transport::TCP);

    table.build::<Connection, Response, Disconnect>().unwrap()
}

#[derive(Serialize, Deserialize, Debug)]
struct Connection {

}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    map: crate::map::Map,
}

#[derive(Serialize, Deserialize, Debug)]
struct Disconnect {

}