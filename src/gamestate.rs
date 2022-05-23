mod mainmenu;

use crate::{map::asteroid, player, networking};

use bevy::prelude::*;
use iyes_loopless::prelude::*;

//new idea for representing game state:
//multiple levels of state: 1. level is GameState, other levels define other state when in certain GameState
//example: RunningState - used when in GameState::Running
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    /// Game setup - loading assets, other stuff needed by both Client and Server ...
    /// Switches to MainMenu when completed
    Loading,
    /// Contains "join server" and "start server"
    /// "join server" switches to ClientSetup
    /// "start server" switches to ServerSetup
    MainMenu,
    /// Handles connecting to the server and downloading the map and preparing everything. Changes to Running when complete.
    ClientSetup,
    /// Loads/generates map. Changes to Running and adds Server to resources when complete.
    ServerSetup,
    /// Everything is loaded and simulation is running
    /// This runs both on server and client
    Running,
}

//pub enum RunningState {
//    Alive,
//    PauseMenu,
//}

//#[derive(Debug, Clone, Eq, PartialEq, Hash)]
//pub enum GameStateOld {
//    /// Game setup - loading assets, other stuff needed by both Client and Server ...
//    /// Switches to MainMenu when completed
//    Loading,
//    /// Contains "join server" and "start server"
//    /// "join server" switches to Client
//    /// "start server" switches to ServerSetup
//    MainMenu,
//    /// Everything is loaded and simulation is running
//    /// This runs both on server and client
//    /// Client uses this to predict movement of objects to reduce trafic
//    Running,
//
//    //SERVER SPECIFIC
//
//    /// Talks to all connected clients and syncs with them  -> instead run when Server resource exists
//    Server,
//
//    //CLIENT SPECIFIC
//    /// Talks to the connected server and syncs with it -> instead run when Client resource exists
//    Client,
//    /// Runs after Client connects to server (and server instructs client to load map?)
//    /// and downloads information from the server to create a copy of the map
//    ClientLoadMap,
//    /// Displays the game, handles user input - maybe split to InGame and Alive  -> Alive handles player movement?
//    InGame,
//    /// Overlay over InGame
//    PauseMenu,
//
//}

/// Registers systems specific to each [`GameState`]
pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        //TODO: use iyes_loopless instead of current Stage and State API.
        //It will be replaced by Stageless RFC https://github.com/bevyengine/rfcs/pull/45

        app

        .add_loopless_state(GameState::Loading)

        //GameState::Loading
        .add_enter_system(GameState::Loading, asteroid::start_loading)
        .add_system_set(
            ConditionSet::new()
            .run_in_state(GameState::Loading)
            .with_system(asteroid::wait_for_load)
            .into()
        )

        //GameState::MainMenu
        .add_system_set(
            ConditionSet::new()
            .run_in_state(GameState::MainMenu)
            .with_system(mainmenu::ui)
            .into()
        )

        //GameState::ClientSetup TODO:
        .add_enter_system(GameState::ClientSetup,networking::client::create_client)
        .add_system_set(
            ConditionSet::new()
            .run_in_state(GameState::ClientSetup)
            .with_system(change_state(GameState::Running))
            .into()
        )

        //GameState::ServerSetup
        .add_enter_system(GameState::ServerSetup,crate::map::generate_map.label("GameState::ServerSetup1"))
        .add_enter_system(GameState::ServerSetup,networking::server::create_server.after("GameState::ServerSetup1"))
        .add_system_set(
            ConditionSet::new()
            .run_in_state(GameState::ServerSetup)
            .with_system(change_state(GameState::Running))
            .into()
        )

        //GameState::Running
        .add_enter_system(GameState::Running,crate::setup)
        .add_system_set(
            ConditionSet::new()
            .run_in_state(GameState::Running)
            .with_system(player::movement_system)
            .into()
        )

        //when server exists    TODO: move to server.rs ? or networking.rs ?
        .add_system_set(
            ConditionSet::new()
            .run_if_resource_exists::<carrier_pigeon::Server>()
            .with_system(bevy_pigeon::app::server_tick/*.label(bevy_pigeon::NetLabel)*/)
            .with_system(networking::server::handle_cons/*.after(bevy_pigeon::NetLabel)*/)
            .into()
        )

        //when client exists    TODO: move to client.rs ? or networking.rs ?
        .add_system_set(
            ConditionSet::new()
            .run_if_resource_exists::<carrier_pigeon::Client>()
            .with_system(bevy_pigeon::app::client_tick/*.label(bevy_pigeon::NetLabel)*/)
            .into()
        );
    }
}

fn change_state<S: bevy::ecs::system::Resource + Clone>(state: S) -> impl Fn(Commands) {
    move |mut commands: Commands| {
        commands.insert_resource(NextState(state.clone()));
    }
}

//fn cleanup<C: Component>(
//    mut commands: Commands,
//    q: Query<Entity,With<C>>,
//) {
//    q.for_each(|e| commands.entity(e).despawn_recursive());
//}