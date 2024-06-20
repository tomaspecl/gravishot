// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use bevy::prelude::*;

use bevy_egui::{egui,EguiContexts};

pub fn ui(
    mut ctx: EguiContexts,

    players: Query<(&crate::player::Health, Has<crate::player::LocalPlayer>, &crate::player::Player)>,
) {
    let ctx = ctx.ctx_mut();

    egui::Window::new("Player health").show(ctx, |ui| {
        for (hp, local, player) in &players {
            let text = if local {
                egui::RichText::new(format!("Local player {player:?}: {}",hp.0)).color(egui::Color32::RED)
            }else{
                egui::RichText::new(format!("Player {player:?}: {}",hp.0))
            };
            ui.label(text);
        }
    });
}