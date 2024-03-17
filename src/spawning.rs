// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

/*use crate::networking::server::ServerMarker;
use crate::networking::{ClientMessage, LocalPlayer, PlayerMap};
use crate::input::Inputs;
use crate::player::{make_player, Player, SpawnPlayer};
use crate::bullet::{make_bullet, SpawnBullet};

use bevy_gravirollback::new::*;
use bevy_gravirollback::new::for_user::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy_quinnet::client::Client;*/
use serde::{Serialize, Deserialize};

// Client/(Server with local Client) -> RequestSpawnEvent -> Server -> allocate new RollbackID -> SpawnPlayer Input -> Client -> Spawn

//TODO: maybe each user could allocate its own part of the RollbackID space such that they can choose their own ID for their Spawns
// that way the Client does not need to send RequestSpawn to the Server to allocate
// new unique RollbackID but can choose it themselves without waiting for the Server response in order to spawn the entity
// the Client would only tell the Server which new entity it is spawning and with which RollbackID, the server will check
// if the Client has permission to do that and then send the Spawn to other Clients
// if the Client does not have permission then the Server will send back some sort of an error to tell the Client
// that it should remove its incorrectly spawned entity

// maybe use most significant bits of RollbackID to represent the PlayerID or the adress space
// example:
// player 5: 000101         000...001101            == 1441151880758558733 decimal
//           ^^^^^^         ^^^^^^^^^^^^
//           player id=5    rollback entity id=13
//TODO: maybe gravirollback should be generic over RollbackID types that would be supplied by the user

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EntityType {
    Player, //TODO: maybe not use RequestSpawnEvent for players too?
    Bullet, //TODO: do not create bullets by RequestSpawnEvent, instead go back to Input Buttons::Shoot
}

//TODO: instead use Input to run spawning systems on the Server, they will create Spawn and send it to the Clients
// Input -> handle inputs and spawn entities in RollbackSet::Update, in the begining, send SpawnEvents -> then receive them in HandleIO::Networking and send them
// ServerMessage::StateSummary overwrites state, use a similar message that only appends state, use that for sending updates of Inputs and spawned entities?

// Client/(Server with local Client) -> LocalInputEvent -> handle_local_input_event -> emit UpdateInputEvent -> handle_update_input_event (check conditions and update Rollback<Inputs>)
//                                                                                                                          |--(we are the Server)--> broadcast Input

//TODO: compare these ^ v

//                                                         Server -> receive Client messages -> map RequestSpawn to SpawnEvent --v
// Client/(Server with local Client) -> LocalSpawnEvent -> handle_local_spawn_event --(we are the server)--> emit SpawnEvent -> handle_spawn_event (check spawn conditions and emit Spawn) -> send all Spawn to all Clients
//                                                                                      --(we are a Client)--> send RequestSpawn to the Server
// Client/Server --(RollbackSet::Update)--> handle_spawns

/*
/// this will run everywhere, on the main Update schedule
pub fn handle_local_spawn_event(
    mut reader: EventReader<LocalSpawnEvent>,
    mut writer: EventWriter<RequestSpawnEvent>,
    client: Option<ResMut<Client>>,
    server_marker: Option<Res<ServerMarker>>,
    info: Res<SnapshotInfo>,
    local_player: Res<LocalPlayer>,
) {
    let frame = info.current;

    if server_marker.is_some() {
        writer.send_batch(reader.read().map(|e| RequestSpawnEvent { frame, player: local_player.0, spawn: e.clone() }));
    }else{
        let mut client = client.expect("Client needs to exist when we are not the Server");
        let c = client.connection_mut();
        for e in reader.read() {
            c.try_send_message(ClientMessage::RequestSpawn(frame, e.clone()));
        }
    }
}

/// This event will be handled on the Server
#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct RequestSpawnEvent {
    pub frame: u64,
    pub player: Player,
    pub spawn: EntityType,
}*/