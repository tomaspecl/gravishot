mod mainmenu;
mod spawn_menu;

use crate::{map, player, networking, input, gravity, bullet};
use networking::rollback::RollbackStages;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use iyes_loopless::prelude::*;

//new idea for representing game state:
//multiple levels of state: 1. level is GameState, other levels define other state when in certain GameState
//example: RunningState - used when in GameState::Running
#[derive(Resource, Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    /// Game setup - loading assets, other stuff needed by both Client and Server ...
    /// Switches to MainMenu when completed
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

#[derive(SystemLabel)]
enum LabelSetup {
    Generate,
    ServerStart,
}

#[derive(SystemLabel)]
enum LabelUpdate {
    ClearInput,
    Input,
    PostInput,
}

#[derive(StageLabel)]
struct RollbackStage;

/// Registers systems specific to each [`GameState`] and other related state
pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        //TODO: use iyes_loopless instead of current Stage and State API.
        //It will be replaced by Stageless RFC https://github.com/bevyengine/rfcs/pull/45

        app

        .add_loopless_state(GameState::Loading)

        //GameState::Loading
        .add_enter_system(GameState::Loading, map::asteroid::start_loading)
        .add_system_set(
            ConditionSet::new()
            .run_in_state(GameState::Loading)
            .with_system(map::asteroid::wait_for_load)
            .into()
        )

        //GameState::LoadingDone
        .add_enter_system(GameState::LoadingDone, after_load)

        //GameState::MainMenu
        .add_system_set(
            ConditionSet::new()
            .run_in_state(GameState::MainMenu)
            .with_system(mainmenu::ui)
            .into()
        )

        //GameState::ClientSetup
        .add_enter_system(GameState::ClientSetup,networking::client::connect)
        .add_system(networking::client::on_connect)
        .add_exit_system(GameState::ClientSetup,crate::setup)

        //GameState::ServerSetup
        .add_enter_system(GameState::ServerSetup,map::generate_map.label(LabelSetup::Generate))
        .add_enter_system(GameState::ServerSetup,networking::server::start.label(LabelSetup::ServerStart))
        .add_enter_system(GameState::ServerSetup,change_state(GameState::Running).after(LabelSetup::Generate).after(LabelSetup::ServerStart))
        .add_exit_system(GameState::ServerSetup,crate::setup);

        //GameState::Running and rollback schedules
        if cfg!(not(feature="headless")) {
            app
            .add_system(player::player_control::change_player_control 
                .run_in_state(GameState::Running))
            .add_system(player::player_control::center_cursor
                .run_in_state(GameState::Running)
                .run_if(player::player_control::is_first_person))

            .add_system(input::get_local_input
                .run_in_state(GameState::Running)
                .label(LabelUpdate::Input)
                .after(LabelUpdate::ClearInput))
            .add_system(spawn_menu::ui
                .run_in_state(GameState::Running)
                .run_if_resource_exists::<networking::LocalPlayer>()
                .run_if_not(player::local_player_exists)
                .label(LabelUpdate::Input)
                .after(LabelUpdate::ClearInput));
        }

        //when server exists    TODO: move to server.rs ? or networking.rs ?
        // Talks to all connected clients and syncs with them
        app
        .add_system(networking::server::handle
            .run_if_resource_exists::<networking::server::ServerMarker>()
            .label(LabelUpdate::PostInput)
            .after(LabelUpdate::Input))
        //when client exists    TODO: move to client.rs ? or networking.rs ?
        // Talks to the connected server and syncs with it
        .add_system(networking::client::handle
            .run_if_resource_exists::<networking::client::ClientMarker>()
            .label(LabelUpdate::PostInput)
            .after(LabelUpdate::Input))

        //.add_system(player::spawn_player_system
        //    .run_in_state(GameState::Running)
        //    .label(LabelUpdate::SpawnDespawn)
        //    .after(LabelUpdate::PostInput))

        //make sure that Command buffers are applied before rollback_schedule
        .add_stage_after(
            CoreStage::Update,
            RollbackStage,
            SystemStage::single(networking::rollback::rollback_schedule::<networking::rollback::MyState>
                .run_in_state(GameState::Running))
        );
        
        let mut roll = networking::rollback::RollbackStagesStorage::new();

        roll.get(RollbackStages::CorePreUpdate)
            .add_system(gravity::force_reset)
            .add_system(player::spawn_player_system
                .run_if_resource_exists::<networking::server::ServerMarker>())  //TODO: is it correct?
            .add_system(bullet::spawn_bullet_system
                .run_if_resource_exists::<networking::server::ServerMarker>());

        roll.get(RollbackStages::CoreUpdate)
            .add_system(player::display_events)
            .add_system(player::stand_up.after(player::display_events))
            .add_system(player::player_control::movement_system.after(player::display_events))
            .add_system(player::player_control::read_result_system)
            .add_system(gravity::gravity_system);
        
        roll.get(RollbackStages::CorePostUpdate)
            .add_system(input::clear);

        roll.get(RollbackStages::PhysicsStagesSyncBackend)
            .add_system_set(RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsStages::SyncBackend));
        
        roll.get(RollbackStages::PhysicsStagesStepSimulation)
            .add_system_set(RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsStages::StepSimulation));

        roll.get(RollbackStages::PhysicsStagesWriteback)
            .add_system_set(RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsStages::Writeback));

        roll.get(RollbackStages::PhysicsStagesDetectDespawn)
            .add_system_set(RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsStages::DetectDespawn));

        app.insert_resource(roll);
    }
}

fn change_state<S: bevy::ecs::system::Resource + Clone>(state: S) -> impl Fn(Commands) {
    move |mut commands: Commands| {
        commands.insert_resource(NextState(state.clone()));
    }
}

fn after_load(mut commands: Commands) {
    if cfg!(feature="headless") {
        //TODO: do these have to be here?
        //let player = player::Player(0);
        //commands.insert_resource(networking::LocalPlayer(player));
        //commands.insert_resource(networking::PlayerMap(bevy::utils::HashMap::from([(player,networking::server::ROLLBACK_ID_COUNTER.get_new())])));
        commands.insert_resource(networking::PlayerMap::default());


        commands.insert_resource(networking::server::ServerMarker);
        commands.insert_resource(NextState(GameState::ServerSetup));
    }else{
        commands.insert_resource(NextState(GameState::MainMenu));
    }
}