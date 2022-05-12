mod physics;
mod player;
mod asteroid;

use bevy::prelude::*;
use heron::prelude::*;

use bevy::diagnostic::{LogDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy_inspector_egui::WorldInspectorPlugin;
//use bevy_editor_pls::prelude::*;

use rand::{thread_rng,Rng};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    /// Contains "join server" and "start server"
    MainMenu,
    /// Game setup - loading map, assets, ...
    Loading,
    /// Everything is loaded and simulation is running
    Running,

    //SERVER SPECIFIC
    //ServerSetup,
    //ServerConnection

    //CLIENT SPECIFIC
    //ClientSetup,
    //ClientConnection
    Connecting,
    PauseMenu,

}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            //width: 200.0,
            //height: 100.0,
            //position: Some(Vec2::ZERO),
            title: "GraviShot".to_string(),
            resizable: true,
            cursor_visible: true,
            cursor_locked: false,
            mode: bevy::window::WindowMode::Windowed,
            ..Default::default()
        })
        .insert_resource(AmbientLight {
            color: Color::rgb(1.0,1.0,1.0),
            brightness: 0.2,
        })
        .add_plugins_with(DefaultPlugins, |group| {
            #[cfg(feature="include_assets")] {
                use bevy_embedded_assets::EmbeddedAssetPlugin;
                group.add_before::<bevy::asset::AssetPlugin, _>(EmbeddedAssetPlugin);
            }
            group
        })
        .add_plugin(LogDiagnosticsPlugin { wait_duration: std::time::Duration::from_secs(5), ..Default::default() })
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(PhysicsPlugin::default())
        //.add_plugin(EditorPlugin)
        .add_plugin(WorldInspectorPlugin::default())
        .add_plugin(physics::GravityPlugin)
        //.register_inspectable::<player::Player>()

        .add_state(GameState::Loading)

        .add_system_set(
            SystemSet::on_enter(GameState::Loading)
                .with_system(asteroid::start_loading)
        )
        .add_system_set(
            SystemSet::on_update(GameState::Loading)
                .with_system(asteroid::wait_for_load)
        )

        .add_system_set(
            SystemSet::on_enter(GameState::Running)
                .with_system(setup)
        )
        .add_system_set(
            SystemSet::on_update(GameState::Running)
                .with_system(player::movement_system)
        )

        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asteroids: Res<asteroid::AsteroidAssets>,
) {
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        point_light: PointLight::default(),
        ..Default::default()
    });

    player::spawn_player(&mut commands, &mut meshes, &mut materials);

    let mut rng = thread_rng();

    for _ in 0..4000 {
        let (x, y, z) = (
            rng.gen_range(-50.0..50.0),
            rng.gen_range(-50.0..50.0),
            rng.gen_range(-50.0..50.0),
        );
        
        asteroid::spawn_asteroid(Transform::from_xyz(x,y,z), None, &mut commands, &asteroids)
    }
}
