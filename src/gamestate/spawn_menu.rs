use crate::input::Buttons;
use crate::networking::server::ServerMarker;
use crate::networking::{ClientMessage, LocalPlayer, rollback::Inputs};

use bevy::prelude::*;

use bevy_egui::{egui,EguiContext};

pub fn ui(
    mut ctx: ResMut<EguiContext>,
    mut inputs: ResMut<Inputs>,
    local_player: Res<LocalPlayer>,
    server: Option<Res<ServerMarker>>,
    client: Res<bevy_quinnet::client::Client>,
) {
    let ctx = ctx.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        if ui.button(egui::RichText::new("spawn").font(egui::FontId::proportional(40.0))).clicked() {
            if server.is_some() {
                inputs.0.entry(local_player.0).or_default().buttons.set(Buttons::Spawn);
            }else{
                client.connection().try_send_message(ClientMessage::RequestPlayer);
            }
        }
    });
}