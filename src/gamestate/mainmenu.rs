use super::GameState;
use crate::{networking::{NetConfig, client::ClientMarker, server::{ServerMarker, ROLLBACK_ID_COUNTER}, LocalPlayer, PlayerMap}, player::Player};

use bevy::prelude::*;
use bevy::utils::HashMap;
use iyes_loopless::prelude::*;
use bevy_egui::{egui,EguiContext};

pub fn ui(
    mut commands: Commands,
    mut ctx: ResMut<EguiContext>,
    mut net: ResMut<NetConfig>,
) {
    let ctx = ctx.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label(egui::RichText::new("Main menu").font(egui::FontId::proportional(40.0)));

        ui.text_edit_singleline(&mut net.ip_port);

        if ui.button("join server").clicked() {
            commands.insert_resource(PlayerMap(HashMap::new()));
            commands.insert_resource(NextState(GameState::ClientSetup));
            commands.insert_resource(ClientMarker);
        }
        if ui.button("start server").clicked() {
            let player = Player(0);
            commands.insert_resource(LocalPlayer(player));
            commands.insert_resource(PlayerMap(HashMap::from([(player,ROLLBACK_ID_COUNTER.get_new())])));
            commands.insert_resource(NextState(GameState::ServerSetup));
            commands.insert_resource(ServerMarker);
        }
    });
}