pub mod server;
pub mod client;
pub mod rollback;

use self::rollback::{Rollback, MyState, Snapshots, Snapshot, State};
use crate::player::Player;
use crate::map::Map;

use bevy::prelude::*;
use bevy::utils::HashMap;

use serde::{Serialize, Deserialize};

/// Sent from Client to Server
#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    /// Client wants to connect
    Connect,
    /// Sent when Client wants to spawn its Player
    RequestPlayer,
    /// Sent to the Server to inform of local player Input
    Input(u64, crate::input::Input),
    /// Sent to the Server to correct the State of local player in specified frame
    Correction(u64, State<MyState>),
}

/// Sent from Server to Clients
#[derive(Serialize, Deserialize)]
pub enum ServerMessage {
    /// Init data for the Client, sent by the Server
    ConnectionGranted(Player, Map, Snapshots<MyState>),
    /// Info about newly connected Client sent to all Clients
    Connected(Player, Rollback),
    /// Info about disconnected Client sent to all Clients
    Disconnected(Player),
    //DespawnPlayer(Player),
    /// Sent to the Client to inform of player Input in specified frame
    Input(u64, Player, crate::input::Input),
    /// Sent to the Client when they are sending future Inputs.
    /// Contains the last Server frame
    SlowDown(u64),
    StateSummary(u64, Snapshot<MyState>, PlayerMap)
}

/*
game schedule:
--collect local input
--send input to server
--update state based on input
*/



//when in client mode -> GameState::Running + Client resource (syncs from server) + display game + handle input
//when in server mode -> GameState::Running + Server resource (syncs to clients)
//      when connect localy -> + display game + handle input, no Client resource because it doesnt sync from server (we are the server)

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
pub struct NetConfig {
    pub ip_port: String,
}

#[derive(Resource, Reflect, Default, Clone, Copy)]
#[reflect(Resource)]
pub struct LocalPlayer(pub Player);

/// Map from Player
#[derive(Resource, Reflect, Serialize, Deserialize, Default, Clone)]
#[reflect(Resource)]
pub struct PlayerMap(pub HashMap<Player, Rollback>);

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(NetConfig {
            ip_port: "localhost:12345".to_string(),
        })
        .insert_resource(Snapshots::<MyState> {  //TODO: maybe instead insert at ConnectionGranted?
            buffer: [Snapshot::default()].into(),
            frame: 0,
            last_frame_time: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH).expect("since UNIX_EPOCH")
                .as_millis(),
        })
        .init_resource::<rollback::Inputs>()
        .register_type::<NetConfig>()
        .register_type::<LocalPlayer>()
        .register_type::<PlayerMap>()
        .register_type::<EntityType>()
        .register_type::<Rollback>()
        .register_type::<rollback::Inputs>()
        //.register_type::<Snapshots::<MyState>>()
        .add_plugin(bevy_quinnet::client::QuinnetClientPlugin::default())
        .add_plugin(bevy_quinnet::server::QuinnetServerPlugin::default());
    }
}

/// Every entity with Rollback component will contain this.
/// Networking code can then use Query<(&Rollback,&EntityType)> to get list
/// of all Rollback entities and work with them.
#[derive(Component, Reflect, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum EntityType {
    Player(Player),    //TODO: either dont use as component or dont include duplicated data
    Bullet,             // -> otherwise player will contain EntityType::Player(Player(id)) and Player(id)
}