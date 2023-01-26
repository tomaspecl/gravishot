mod physics;
mod player;
mod gamestate;
mod networking;
mod map;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

fn main() {
    let default_plugins = {
        #[allow(unused_mut)]
        let mut p = DefaultPlugins.build()
        .set(WindowPlugin {
            window: WindowDescriptor {
                //width: 200.0,
                //height: 100.0,
                //position: Some(Vec2::ZERO),
                title: "GraviShot".to_string(),
                resizable: true,
                cursor_visible: true,
                cursor_grab_mode: bevy::window::CursorGrabMode::Locked,
                mode: bevy::window::WindowMode::Windowed,
                ..Default::default()
            },
            ..Default::default()
        });

        #[cfg(feature="include_assets")] {
            p = p.add_before::<bevy::asset::AssetPlugin, _>(bevy_embedded_assets::EmbeddedAssetPlugin);
        }
        p
    };

    App::new()
    .insert_resource(AmbientLight {
        color: Color::rgb(1.0,1.0,1.0),
        brightness: 0.2,
    })
    .add_plugins(default_plugins)
    /*.add_plugin(bevy::diagnostic::LogDiagnosticsPlugin {
        wait_duration: std::time::Duration::from_secs(5),
        ..Default::default()
    })
    .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())*/
    .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
    .add_plugin(bevy_egui::EguiPlugin)
    //.add_plugin(EditorPlugin)

    .add_plugin(RapierDebugRenderPlugin::default())

    //.add_plugin(bevy_inspector_egui_rapier::InspectableRapierPlugin)
    .add_plugin(bevy_inspector_egui::quick::WorldInspectorPlugin)

    .add_plugin(player::PlayerPlugin)
    .add_plugin(physics::GravityPlugin)
    .add_plugin(gamestate::GameStatePlugin)
    .add_plugin(networking::NetworkPlugin)
    
    //.add_state(gamestate::GameState::Loading)
    .add_system(bevy::window::close_on_esc)

    .run();
}

fn setup(
    mut commands: Commands,
    asteroids: Res<map::asteroid::AsteroidAssets>,
    map: Res<map::Map>,
) {
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        point_light: PointLight::default(),
        ..Default::default()
    });

    map::load_from_map(commands, map, asteroids);
}
