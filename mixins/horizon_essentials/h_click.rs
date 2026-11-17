// Un clic sur la page de réglages : l'interface le traduit en Action, le Core l'applique.
fn essentials_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::{essentials_events as ev, essentials_settings as ui, essentials_types as t};
    if mod_id != t::MOD_ID {
        return;
    }
    if let Some(action) = ui::click(button) {
        let game = &*game;
        ev::apply(|| game.native_data_dir(t::MOD_ID), action);
    }
}
