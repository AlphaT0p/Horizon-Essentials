// La page Options > Mods > Horizon Essentials > Settings : ses libellés.
fn essentials_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::{essentials_events as ev, essentials_settings as ui, essentials_types as t};
    if mod_id != t::MOD_ID {
        return;
    }
    let game = &*game;
    ev::view(|| game.native_data_dir(t::MOD_ID), |resolved, _| ui::labels(resolved, buttons));
}
