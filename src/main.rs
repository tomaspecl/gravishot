// Gravishot
// Copyright (C) 2023 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

mod gravity;
mod player;
mod gamestate;
mod networking;
mod map;
mod input;
mod bullet;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

fn main() {
    let mut app = App::new();

    #[allow(unused_mut)]
    let mut default_plugins = DefaultPlugins.build();

    if cfg!(feature="headless") {
        default_plugins = default_plugins.set(WindowPlugin {
            primary_window: None,
            exit_condition: bevy::window::ExitCondition::DontExit,
            close_when_requested: true,
        })
        .disable::<bevy::winit::WinitPlugin>()
        .set(bevy::render::RenderPlugin {
            wgpu_settings: bevy::render::settings::WgpuSettings {
                backends: None,
                ..default()
            },
        });
    }else{
        default_plugins = default_plugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "GraviShot".to_string(),
                resizable: true,
                cursor: bevy::window::Cursor {
                    visible: true,
                    grab_mode: bevy::window::CursorGrabMode::Locked,
                    ..default()
                },
                mode: bevy::window::WindowMode::Windowed,
                ..default()
            }),
            ..default()
        });
        app.insert_resource(AmbientLight {
            color: Color::rgb(1.0,1.0,1.0),
            brightness: 0.2,
        });
    }

    #[cfg(feature="include_assets")] {
        default_plugins = default_plugins.add_before::<bevy::asset::AssetPlugin, _>(bevy_embedded_assets::EmbeddedAssetPlugin);
    }
    
    app.add_plugins(default_plugins);

    if cfg!(feature="headless") { 
        app.add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(bevy::utils::Duration::from_secs_f64(
            1.0 / 60.0  //TODO: figure out why the server laggs behind when there is no wait_duration
        )));
    }else{
        app.add_plugins((
            bevy_egui::EguiPlugin,
            bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
            //bevy_inspector_egui_rapier::InspectableRapierPlugin,  //TODO: is it still needed?
            //EditorPlugin,
            //RapierDebugRenderPlugin::default(),
        ));
    }

    /*
    app.add_plugins((
        bevy::diagnostic::LogDiagnosticsPlugin {
            wait_duration: std::time::Duration::from_secs(5),
            ..default()
        },
        bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
    ));     // */

    app.add_plugins((
        RapierPhysicsPlugin::<NoUserData>::default()
            .with_default_system_setup(false),
        player::PlayerPlugin,
        gravity::GravityPlugin,
        gamestate::GameStatePlugin,
        networking::NetworkPlugin,
    ))
    
    .add_systems(Update,bevy::window::close_on_esc)

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
        ..default()
    });

    map::load_from_map(commands, map, asteroids);
}
