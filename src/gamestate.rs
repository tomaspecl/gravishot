mod mainmenu;

use crate::{map::asteroid, player, networking};

use bevy::prelude::*;
use iyes_loopless::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    /// Game setup - loading assets, other stuff needed by both Client and Server ...
    /// Switches to MainMenu when completed
    Loading,
    /// Contains "join server" and "start server"
    /// "join server" switches to Client
    /// "start server" switches to ServerSetup
    MainMenu,
    /// Everything is loaded and simulation is running
    /// This runs both on server and client
    /// Client uses this to predict movement of objects to reduce trafic
    Running,

    //SERVER SPECIFIC
    /// Loads/generates map, when complete, changes to Running and adds Server to game state
    //ServerSetup,      instead move to on_enter(Server)    //TODO: probably needed
    /// Talks to all connected clients and syncs with them  -> instead run when Server resource exists
    Server,

    //CLIENT SPECIFIC
    /// Talks to the connected server and syncs with it -> instead run when Client resource exists
    Client,
    /// Runs after Client connects to server (and server instructs client to load map?)
    /// and downloads information from the server to create a copy of the map
    ClientLoadMap,
    /// Displays the game, handles user input - maybe split to InGame and Alive  -> Alive handles player movement?
    InGame,
    /// Overlay over InGame
    PauseMenu,

}

/// Registers systems specific to each [`GameState`]
pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {

        //TODO: when need to update systems when they are inside the state stack and not only when on top of it
        //you have to use SystemSet::on_in_stack_update()

        //TODO: use iyes_loopless instead of current Stage and State API.
        //It will be replaced by Stageless RFC https://github.com/bevyengine/rfcs/pull/45

        app

        .add_system(gamestate_changer)

        .add_system_set(
            SystemSet::on_enter(GameState::Loading)
                .with_system(asteroid::start_loading)
        )
        .add_system_set(
            SystemSet::on_update(GameState::Loading)
                .with_system(asteroid::wait_for_load)
        );

        mainmenu::register_systems(app);

        networking::server::register_systems(app);

        networking::client::register_systems(app);

        //TODO: every system under this comment needs to be reworked/ported
        /*.add_system_set(
            SystemSet::on_enter(GameState::Running)
                .with_system(bevy_pigeon::app::comp_recv::<Transform, NetTransform>.after(setup_game)),
        )*/
        app
        .add_system_set(
            SystemSet::on_update(GameState::Running)  
        )

        .add_system_set(
            SystemSet::on_enter(GameState::Running)
                .with_system(crate::setup)
        )
        .add_system_set(
            SystemSet::on_update(GameState::Running)
                .with_system(player::movement_system)
        );
    }
}

struct ChangeState {
    set_first: bool,
    states: Vec<GameState>,
}

impl ChangeState {
    pub fn new(set_first: bool,states: Vec<GameState>) -> ChangeState {
        ChangeState {
            set_first,
            states,
        }
    }
}

fn gamestate_changer(
    mut commands: Commands,
    mut state: ResMut<State<GameState>>,
    change: Option<ResMut<ChangeState>>,
) {
    if let Some(mut change) = change {
        if let Some(s) = change.states.pop() {
            if change.set_first {
                change.set_first = false;
                println!("gamestate change set {s:?}");
                state.set(s).unwrap();
            }else{
                println!("gamestate change push {s:?}");
                state.push(s).unwrap();
            }
        }else{
            commands.remove_resource::<ChangeState>();
        }
    }
}

fn cleanup<C: Component>(
    mut commands: Commands,
    q: Query<Entity,With<C>>,
) {
    q.for_each(|e| commands.entity(e).despawn_recursive());
}