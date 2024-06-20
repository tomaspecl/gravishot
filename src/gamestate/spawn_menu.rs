// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::networking::rollback::ROLLBACK_ID_COUNTER;

use bevy::prelude::*;

use bevy_egui::{egui,EguiContexts};

pub fn ui(
    mut ctx: EguiContexts,
    //mut events: EventWriter<crate::spawning::LocalSpawnEvent>,
    mut local_input: ResMut<crate::input::LocalInput>,
) {
    let ctx = ctx.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        if ui.button(egui::RichText::new("spawn").font(egui::FontId::proportional(40.0))).clicked() {
            local_input.0.signals.spawn = Some(crate::input::PlayerSpawnSignal {
                body: ROLLBACK_ID_COUNTER.get_new(),
                gun: ROLLBACK_ID_COUNTER.get_new(),
            });
        }
    });
}