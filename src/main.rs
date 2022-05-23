mod physics;
mod player;
mod gamestate;
mod networking;
mod map;

use bevy::prelude::*;
use heron::prelude::*;

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
        #[cfg(feature="include_assets")]
        group.add_before::<bevy::asset::AssetPlugin, _>(bevy_embedded_assets::EmbeddedAssetPlugin);
        group
    })
    /*.add_plugin(bevy::diagnostic::LogDiagnosticsPlugin {
        wait_duration: std::time::Duration::from_secs(5),
        ..Default::default()
    })
    .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())*/
    .add_plugin(PhysicsPlugin::default())
    .add_plugin(bevy_egui::EguiPlugin)
    //.add_plugin(EditorPlugin)
    .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default())

    .add_plugin(physics::GravityPlugin)
    .add_plugin(gamestate::GameStatePlugin)
    .add_plugin(networking::NetworkPlugin)

    //.add_state(gamestate::GameState::Loading)

    .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asteroids: Res<map::asteroid::AsteroidAssets>,
    map: Res<map::Map>,
) {
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        point_light: PointLight::default(),
        ..Default::default()
    });

    player::spawn_player(&mut commands, &mut meshes, &mut materials);

    map::load_from_map(commands, map, asteroids)
}
