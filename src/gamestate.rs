// Gravishot
// Copyright (C) 2023 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

mod mainmenu;
mod spawn_menu;

use crate::{map, player, networking, input, gravity, bullet};

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

//new idea for representing game state:
//multiple levels of state: 1. level is GameState, other levels define other state when in certain GameState
//example: RunningState - used when in GameState::Running
#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    /// Game setup - loading assets, other stuff needed by both Client and Server ...
    /// Switches to MainMenu when completed
    #[default]
    Loading,
    LoadingDone,
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

#[derive(SystemSet, Hash, Debug, PartialEq, Eq, Clone)]
enum RollbackSet {
    Input,
    RunNetworking,
    RunRollback,
}

/// Registers systems specific to each [`GameState`] and other related state
pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        //TODO: use iyes_loopless instead of current Stage and State API.
        //It will be replaced by Stageless RFC https://github.com/bevyengine/rfcs/pull/45

        app

        .add_state::<GameState>()

        //GameState::Loading
        .add_systems(OnEnter(GameState::Loading),map::asteroid::start_loading)
        .add_systems(Update,map::asteroid::wait_for_load.run_if(in_state(GameState::Loading)))

        //GameState::LoadingDone
        .add_systems(OnEnter(GameState::LoadingDone),after_load)

        //GameState::MainMenu
        .add_systems(Update,mainmenu::ui.run_if(in_state(GameState::MainMenu)))

        //GameState::ClientSetup
        .add_systems(OnEnter(GameState::ClientSetup),networking::client::connect)
        .add_systems(Update,networking::client::on_connect.run_if(in_state(GameState::ClientSetup)))
        .add_systems(OnExit(GameState::ClientSetup),crate::setup)

        //GameState::ServerSetup
        .add_systems(OnEnter(GameState::ServerSetup),
            (
                (map::generate_map,networking::server::start),
                change_state(GameState::Running),
            ).chain()
        )
        .add_systems(OnExit(GameState::ServerSetup),crate::setup);

        //GameState::Running and rollback schedules
        if cfg!(not(feature="headless")) {
            app
            .add_systems(Update,
                (
                    player::player_control::change_player_control,
                    player::player_control::center_cursor.run_if(player::player_control::is_first_person),
                    (
                        input::get_local_input,     //after clear input
                        spawn_menu::ui              //after clear input
                            .run_if(resource_exists::<networking::LocalPlayer>())
                            .run_if(not(player::local_player_exists))
                    ).in_set(RollbackSet::Input)
                )
                .run_if(in_state(GameState::Running))
            );
        }
        
        app
        .configure_sets(Update, (RollbackSet::Input,RollbackSet::RunNetworking,RollbackSet::RunRollback).chain())
        .add_systems(Update,
            (
                //when server exists    TODO: move to server.rs ? or networking.rs ?
                // Talks to all connected clients and syncs with them
                networking::server::handle.run_if(resource_exists::<networking::server::ServerMarker>()),
                //when client exists    TODO: move to client.rs ? or networking.rs ?
                // Talks to the connected server and syncs with it
                networking::client::handle.run_if(resource_exists::<networking::client::ClientMarker>()),
            ).in_set(RollbackSet::RunNetworking)
        );

        //.add_system(player::spawn_player_system
        //    .run_in_state(GameState::Running)
        //    .label(LabelUpdate::SpawnDespawn)
        //    .after(LabelUpdate::PostInput))

        let mut schedule = Schedule::new();
        schedule.add_systems(
            (
                (   //CorePreUpdate
                    gravity::force_reset,
                    player::spawn_player_system
                        .run_if(resource_exists::<networking::server::ServerMarker>()),  //TODO: is it correct? 
                    bullet::spawn_bullet_system
                        .run_if(resource_exists::<networking::server::ServerMarker>()),
                    bullet::despawn_bullet_system
                        .run_if(resource_exists::<networking::server::ServerMarker>()),
                ),
                apply_deferred,
                (   //CoreUpdate
                    player::display_events,
                    player::stand_up.after(player::display_events),
                    player::player_control::movement_system.after(player::display_events),
                    player::player_control::read_result_system,
                    gravity::gravity_system,
                ),
                apply_deferred,
                (   //CorePostUpdate
                    input::clear,
                    (
                        RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::SyncBackend),
                        RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::SyncBackendFlush),
                        RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::StepSimulation),
                        RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::Writeback),
                    )
                    .chain()
                    //.before(bevy::transform::TransformSystem::TransformPropagate)     //TODO: there is no TransformPropagate
                )
            )
            .chain()
        );

        app.add_schedule(networking::rollback::RollbackSchedule, schedule)
            .add_systems(Update,
                (
                    apply_deferred,
                    networking::rollback::run_rollback_schedule::<networking::rollback::MyState>,
                )
                .chain()
                .in_set(RollbackSet::RunRollback)
                .run_if(in_state(GameState::Running))
            );
    }
}

fn change_state<S: States>(state: S) -> impl Fn(ResMut<NextState<S>>) {
    move |mut next_state: ResMut<NextState<S>>| {
        next_state.set(state.clone());
    }
}

fn after_load(
    mut commands: Commands,
    mut state: ResMut<NextState<GameState>>,
) {
    if cfg!(feature="headless") {
        //TODO: do these have to be here?
        //let player = player::Player(0);
        //commands.insert_resource(networking::LocalPlayer(player));
        //commands.insert_resource(networking::PlayerMap(bevy::utils::HashMap::from([(player,networking::server::ROLLBACK_ID_COUNTER.get_new())])));
        commands.insert_resource(networking::PlayerMap::default());

        commands.insert_resource(networking::server::ServerMarker);
        state.set(GameState::ServerSetup);
    }else{
        state.set(GameState::MainMenu);
    }
}
