use bevy::prelude::*;

use bevy_egui::{egui,EguiContext};

pub fn ui(
    mut ctx: ResMut<EguiContext>,
    server: Option<Res<carrier_pigeon::Server>>,
    client: Option<Res<carrier_pigeon::Client>>,
    mut event: EventWriter<crate::player::SpawnPlayerEvent>,
) {
    let ctx = ctx.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        if server.is_some() {
            if ui.button(egui::RichText::new("spawn").font(egui::FontId::proportional(40.0))).clicked() {
                event.send(crate::player::SpawnPlayerEvent {
                    cid: 0,
                    nid: crate::networking::server::NET_ENTITY_ID_COUNTER.get_new(),
                    transform: Transform::from_xyz(100.0,0.0,0.0),
                })
            }
        }else{
            if ui.button(egui::RichText::new("spawn").font(egui::FontId::proportional(40.0))).clicked() {
                let client = client.unwrap();

                client.send(&crate::networking::RequestPlayer).unwrap();
            }
        }
    });
}