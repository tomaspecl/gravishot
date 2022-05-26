pub mod server;
pub mod client;

use bevy::prelude::*;

use bevy_pigeon::{types::NetTransform, AppExt};
use carrier_pigeon::{MsgTable, Transport, MsgTableParts, CId};
use serde::{Serialize, Deserialize};

//when in client mode -> GameState::Running + Client resource (syncs from server) + display game + handle input
//when in server mode -> GameState::Running + Server resource (syncs to clients)
//      when connect localy -> + display game + handle input, no Client resource because it doesnt sync from server (we are the server)

pub struct NetConfig {
    pub ip_port: String,
    pub msg_table: MsgTableParts,
    pub local_player_cid: CId,
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        let config = NetConfig {
            ip_port: "localhost:12345".to_string(),
            msg_table: make_msg_table(app),
            local_player_cid: 0,
        };

        app.insert_resource(config);
    }
}

fn make_msg_table(app: &mut App) -> MsgTableParts {
    let mut table = MsgTable::new();

    table.register::<RequestPlayer>(Transport::TCP).unwrap();
    table.register::<SpawnPlayer>(Transport::TCP).unwrap();
    table.register::<DespawnPlayer>(Transport::TCP).unwrap();

    app.sync_comp::<Transform, NetTransform>(&mut table, Transport::TCP);

    table.build::<Connection, Response, Disconnect>().unwrap()
}

#[derive(Serialize, Deserialize, Debug)]
struct Connection {

}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    cid: CId,
    map: crate::map::Map,
}

#[derive(Serialize, Deserialize, Debug)]
struct Disconnect {

}

/// Sent from Client to Server when Client wants to spawn its Player
#[derive(Serialize, Deserialize, Debug)]
pub struct RequestPlayer;

/// Sent from Server to Clients to inform them of a new Player
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpawnPlayer {
    /// Connection ID
    pub cid: CId,
    /// NetEntity ID
    pub nid: u64,
    /// Position of the Player
    pub transform: NetTransform,
}

impl From<crate::player::SpawnPlayerEvent> for SpawnPlayer {
    fn from(p: crate::player::SpawnPlayerEvent) -> Self {
        Self {
            cid: p.cid,
            nid: p.nid,
            transform: p.transform.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DespawnPlayer(CId);

/// Every entity with NetEntity component will contain this.
/// Networking code can then use Query<(&NetEntity,&NetMarker)> to get list of all NetEntities
/// and work with them.
#[derive(Component)]
pub enum NetMarker {
    Player,
    Bullet,
}