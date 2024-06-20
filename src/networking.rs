// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

pub mod server;
pub mod client;
pub mod rollback;

use crate::input::{UpdateInputEvent, LocalInput, Input, Inputs};
use crate::player::Player;
use crate::map::Map;
use rollback::UpdateStateEvent;
use rollback::{State, States, Snapshot, PhysicsBundle};

use bevy_gravirollback::new::*;
use bevy_gravirollback::new::systems::*;

use bevy::prelude::*;

use serde::{Serialize, Deserialize};

/// Sent from Client to Server
#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    /// Client wants to connect
    Connect,
    /// Sent to the Server to inform of local player Input
    Input(u64, Input),
    /// Sent to the Server to correct the State of local player in specified frame
    Correction(u64, State),
}

/// Sent from Server to Clients
#[derive(Serialize, Deserialize)]
pub enum ServerMessage {
    /// Init data for the Client, sent by the Server
    ConnectionGranted(Player, Map, States),
    /// Info about newly connected Client sent to all Clients
    Connected(Player),
    /// Info about disconnected Client sent to all Clients
    Disconnected(Player),
    //DespawnPlayer(Player),
    /// Sent to the Client to inform of player Input in specified frame
    Input(UpdateInputEvent),
    /// Sent to the Client when they are sending future Inputs.
    /// Contains the last Server frame
    SlowDown(u64),
    StateSummary(u64, Snapshot),
    MapUpdate(Map),
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
pub struct LocalPlayer(pub Player); //TODO: is this needed?

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(NetConfig {
            ip_port: "localhost:12345".to_string(),
        })
        .add_event::<UpdateInputEvent>()
        .add_event::<UpdateStateEvent<State>>()
        .init_resource::<Inputs>()
        .init_resource::<LocalInput>()
        .init_resource::<Rollback<Inputs>>()
        .init_resource::<crate::map::Map>()
        .register_type::<NetConfig>()
        .register_type::<LocalPlayer>()
        .register_type::<crate::player::PlayerParts>()
        .register_type::<EntityType>()
        .register_type::<RollbackID>()
        .register_type::<Inputs>()
        .register_type::<LocalInput>()
        .register_type::<SnapshotInfo>()
        .register_type::<RollbackMap>()
        .register_type::<Rollback<Inputs>>()
        .register_type::<Rollback<PhysicsBundle>>()
        .register_type::<Rollback<crate::player::HeadData>>()
        .register_type::<crate::map::Map>()
        //.register_type::<Snapshots::<MyState>>()
        .add_plugins((
            bevy_quinnet::client::QuinnetClientPlugin {
                initialize_later: true,
            },
            bevy_quinnet::server::QuinnetServerPlugin {
                initialize_later: true,
            },
            bevy_gravirollback::new::RollbackPlugin::default(),
        ))

        //.insert_resource(RollbackRegistry {
        //    getters: vec![getter::<Transform>, getter::<Velocity>],
        //})
        
        //TODO: this is not really networking
        .add_systems(RollbackSchedule,(
            PhysicsBundle::get_default_rollback_systems(),
            crate::player::Health::get_default_rollback_systems(),
            crate::player::HeadData::get_default_rollback_systems(),
            restore_resource::<Inputs>.in_set(RollbackSet::RestoreInputs),
        ));
    }
}

/// Every entity with Rollback component will contain this.
/// Networking code can then use Query<(&Rollback,&EntityType)> to get list
/// of all Rollback entities and work with them.
#[derive(Component, Reflect, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum EntityType {
    Player(Player),    //TODO: either dont use as component or dont include duplicated data
    Gun(Option<Player>),
    Bullet,             // -> otherwise player will contain EntityType::Player(Player(id)) and Player(id)
}