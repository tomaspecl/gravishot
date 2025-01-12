// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

mod mainmenu;
mod spawn_menu;
mod healthbar;

use crate::{map, player, networking, input, gravity, bullet, physics};

use bevy_gravirollback::prelude::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
pub enum HandleIO {
    LocalInput,
    Networking,
    ProcessChanges,
}

/// Registers systems specific to each [`GameState`] and other related state
pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        //TODO: use iyes_loopless instead of current Stage and State API.
        //It will be replaced by Stageless RFC https://github.com/bevyengine/rfcs/pull/45

        app
        .insert_resource({
            UpdateTimer {
                delay: gravity::PHYSICS_TIMESTEP_MS,
                frame_0_time: Duration::from_secs(0),
            }
        })
        .register_type::<UpdateTimer>()

        .init_state::<GameState>()

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
        .add_systems(OnExit(GameState::ServerSetup),(crate::setup_server, crate::setup))

        .configure_sets(Update,
            (HandleIO::LocalInput, HandleIO::Networking, HandleIO::ProcessChanges).chain().in_set(RollbackProcessSet::HandleIO)
        );

        //GameState::Running and rollback schedules
        if cfg!(not(feature="headless")) {
            app
            .add_systems(Update,
                (
                    player::player_control::change_player_control,
                    player::player_control::center_cursor.run_if(player::player_control::is_first_person),

                    (
                        input::get_local_input.run_if(GAME_TICK_CONDITION),

                        spawn_menu::ui.run_if(not(player::local_player_exists)),
                        healthbar::ui.run_if(player::local_player_exists),
                    ).in_set(HandleIO::LocalInput),

                    (
                        input::handle_local_input_event.run_if(GAME_TICK_CONDITION),
                        /*(
                            spawning::handle_local_spawn_event,
                            spawning::handle_request_spawn_event.run_if(resource_exists::<networking::server::ServerMarker>()),
                        ).chain(),*/
                    ).in_set(HandleIO::Networking),
                ).run_if(in_state(GameState::Running))
            );
        }
        
        //taken from bevy_rapier3d::plugin:
        // These *must* be in the main schedule currently so that they do not miss events.
        app.add_systems(PostUpdate, (bevy_rapier3d::plugin::systems::sync_removals,));
        
        app
        .add_systems(Update,(   //TODO: put this into GameState::Running, client::handle part that handles connecting should be before GameState::Running
            (
                //when server exists    TODO: move to server.rs ? or networking.rs ?
                // Talks to all connected clients and syncs with them
                (
                    networking::server::handle,
                    networking::server::send_state_summary,
                ).run_if(resource_exists::<networking::server::ServerMarker>),
                //when client exists    TODO: move to client.rs ? or networking.rs ?
                // Talks to the connected server and syncs with it
                networking::client::handle.run_if(resource_exists::<networking::client::ClientMarker>),
            ).in_set(HandleIO::Networking),

            map::load_from_map.run_if(in_state(GameState::Running)),

            (
                (
                    update_frame,
                    input::handle_update_input_event,
                    //spawning::handle_update_spawn_event,
                    networking::rollback::handle_update_state_event,
                ).in_set(HandleIO::ProcessChanges),
            ).run_if(in_state(GameState::Running)),
        ));

        app
        .add_systems(RollbackUpdate,
            (
                (   //CorePreUpdate
                    gravity::force_reset,
                    //spawning::handle_spawns,
                    player::spawn_player_system,
                    player::health_system,
                    (
                        player::gun::connect_joints,
                        player::gun::update_joints,
                        bullet::spawn_bullet_system
                    ).chain(),
                    (
                        bullet::bullet_collision_system,
                        bullet::despawn_bullet_system,
                        physics::physics_body_existance_system,
                    ).chain(),
                ),
                apply_deferred,
                (   //CoreUpdate
                    //player::display_events,
                    //player::stand_up.after(player::display_events),
                    (
                        gravity::gravity_system,
                        player::player_control::movement_system,
                        //player::display_events,
                    ).chain(),
                    player::player_control::read_result_system,
                ),
                apply_deferred,
                (   //CorePostUpdate
                    (
                        RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::SyncBackend),
                        RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::StepSimulation),
                        RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::Writeback),
                        (bevy::transform::systems::propagate_transforms, bevy::transform::systems::sync_simple_transforms),
                    ).chain(),
                    bevy_rapier3d::plugin::systems::sync_removals,
                )
            ).chain().in_set(RollbackUpdateSet::Update)
        )
        .configure_sets(Update, RollbackProcessSet::RunRollbackSchedule.run_if(in_state(GameState::Running)));
    }
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
pub struct UpdateTimer {
    pub delay: u64,
    pub frame_0_time: Duration,
}

const GAME_TICK_CONDITION: for<'a> fn(Res<'a, WantedFrame>) -> bool = resource_changed::<WantedFrame>;

fn update_frame(
    timer: Res<UpdateTimer>,
    mut wanted: ResMut<WantedFrame>,
) {
    let delay = timer.delay;
    let frame0 = timer.frame_0_time;
    let needed_frame = ((SystemTime::now().duration_since(UNIX_EPOCH).unwrap() - frame0).as_millis() / delay as u128) as u64;
    if needed_frame > wanted.0 {
        wanted.as_mut().0 += 1;
    }
}

fn change_state<S: bevy::state::state::FreelyMutableState>(state: S) -> impl Fn(ResMut<NextState<S>>) {
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

        commands.insert_resource(networking::server::ServerMarker);
        state.set(GameState::ServerSetup);
    }else{
        state.set(GameState::MainMenu);
    }
}
