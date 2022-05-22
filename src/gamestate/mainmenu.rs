use super::{GameState, ChangeState};
use crate::networking::NetConfig;

use bevy::prelude::*;
use bevy_egui::{egui,EguiContext};

pub(crate) fn register_systems(app: &mut App) {
    app
    .add_system_set(
        SystemSet::on_update(GameState::MainMenu)
            .with_system(ui)
    );
}

#[derive(Component)]
struct StateHolder(GameState);

fn ui(
    mut commands: Commands,
    mut ctx: ResMut<EguiContext>,
    mut net: ResMut<NetConfig>,
    mut state: ResMut<State<GameState>>,
) {
    let ctx = ctx.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label(egui::RichText::new("Main menu").font(egui::FontId::proportional(40.0)));

        ui.text_edit_singleline(&mut net.ip_port);

        if ui.button("join server").clicked() {
            commands.insert_resource(ChangeState::new(true,vec![
                GameState::Running,
                GameState::Client,
            ]));
        }
        if ui.button("start server").clicked() {
            //state.set(GameState::ServerSetup).unwrap();
            commands.insert_resource(ChangeState::new(true,vec![
                GameState::Running,
                GameState::Server,
            ]));
        }
    });
}