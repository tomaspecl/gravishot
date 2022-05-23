use super::GameState;
use crate::networking::NetConfig;

use bevy::prelude::*;
use iyes_loopless::prelude::*;
use bevy_egui::{egui,EguiContext};

#[derive(Component)]
struct StateHolder(GameState);

pub fn ui(
    mut commands: Commands,
    mut ctx: ResMut<EguiContext>,
    mut net: ResMut<NetConfig>,
    //mut state: ResMut<State<GameState>>,
) {
    let ctx = ctx.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label(egui::RichText::new("Main menu").font(egui::FontId::proportional(40.0)));

        ui.text_edit_singleline(&mut net.ip_port);

        if ui.button("join server").clicked() {
            commands.insert_resource(NextState(GameState::ClientSetup));
        }
        if ui.button("start server").clicked() {
            commands.insert_resource(NextState(GameState::ServerSetup));
        }
    });
}