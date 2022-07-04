mod mainmenu;
mod spawn_menu;

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
    /// Client uses this to predict movement of objects to reduce trafic
    Running,
}

//pub enum RunningState {
      /// Player is alive - display the game and handle user input
//    Alive,
      /// Overlay over InGame
//    PauseMenu,
//}

/// Registers systems specific to each [`GameState`] and other related state
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

        //GameState::ClientSetup
        .add_enter_system(GameState::ClientSetup,networking::client::create_client.label("ClientSetup"))
        .add_enter_system(GameState::ClientSetup,change_state(GameState::Running).after("ClientSetup"))
        .add_exit_system(GameState::ClientSetup,crate::setup)

        //GameState::ServerSetup
        .add_enter_system(GameState::ServerSetup,crate::map::generate_map.label("ServerSetup1"))
        .add_enter_system(GameState::ServerSetup,networking::server::create_server.label("ServerSetup2"))
        .add_enter_system(GameState::ServerSetup,change_state(GameState::Running).after("ServerSetup1").after("ServerSetup2"))
        .add_exit_system(GameState::ServerSetup,crate::setup)

        //GameState::Running
        .add_system_set(
            ConditionSet::new()
            .run_in_state(GameState::Running)
            .with_system(player::movement_system)
            .with_system(player::spawn_player_event_handler)
            .with_system(player::despawn_player_event_handler)
            .with_system(player::display_events)
            .with_system(player::stand_up)
            .into()
        )
        .add_system_set(
            ConditionSet::new()
            .run_in_state(GameState::Running)
            .run_if_not(player::local_player_exists)
            .with_system(spawn_menu::ui)
            .into()
        )

        //when server exists    TODO: move to server.rs ? or networking.rs ?
        // Talks to all connected clients and syncs with them
        .add_system_set(
            ConditionSet::new()
            .run_if_resource_exists::<carrier_pigeon::Server>()
            .with_system(bevy_pigeon::app::server_tick/*.label(bevy_pigeon::NetLabel)*/)
            .with_system(networking::server::handle_cons/*.after(bevy_pigeon::NetLabel)*/)
            .with_system(networking::server::send_player_spawns)
            .with_system(networking::server::handle_player_spawn_requests)
            .into()
        )

        //when client exists    TODO: move to client.rs ? or networking.rs ?
        // Talks to the connected server and syncs with it
        .add_system_set(
            ConditionSet::new()
            .run_if_resource_exists::<carrier_pigeon::Client>()
            .with_system(bevy_pigeon::app::client_tick/*.label(bevy_pigeon::NetLabel)*/)
            .with_system(networking::client::receive_player_spawns)
            .with_system(networking::client::receive_player_despawns)
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