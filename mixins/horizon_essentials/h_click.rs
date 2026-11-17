// Un clic sur la page de réglages : l'interface le traduit en Action, le Core l'applique.
// Pendant le chargement de la configuration, un clic ne fait rien.
fn essentials_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::{essentials_events as ev, essentials_settings as ui, essentials_types as t};
    if mod_id != t::MOD_ID {
        return;
    }
    let game = &*game;
    let dir = || game.native_data_dir(t::MOD_ID);
    let loaded = ev::view(dir, |snapshot| snapshot.is_some()).unwrap_or(false);
    if let Some(action) = ui::click(button, loaded) {
        ev::apply(dir, action);
    }
}
