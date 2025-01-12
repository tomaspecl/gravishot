// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use super::GameState;
use crate::networking::{NetConfig, client::ClientMarker, server::ServerMarker, LocalPlayer};
use crate::player::Player;

use bevy::prelude::*;
use bevy_egui::{egui,EguiContexts};

pub fn ui(
    mut commands: Commands,
    mut state: ResMut<NextState<GameState>>,
    mut ctx: EguiContexts,
    mut net: ResMut<NetConfig>,
) {
    let ctx = ctx.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label(egui::RichText::new("Main menu").font(egui::FontId::proportional(40.0)));

        ui.text_edit_singleline(&mut net.ip_port);

        if ui.button("join server").clicked() {
            commands.init_resource::<bevy_quinnet::client::QuinnetClient>();
            commands.insert_resource(ClientMarker);
            state.set(GameState::ClientSetup);
        }
        if ui.button("start server").clicked() {
            let player = Player(0);
            commands.insert_resource(LocalPlayer(player));
            commands.init_resource::<bevy_quinnet::server::QuinnetServer>();
            commands.insert_resource(ServerMarker);
            state.set(GameState::ServerSetup);
        }
    });
}
