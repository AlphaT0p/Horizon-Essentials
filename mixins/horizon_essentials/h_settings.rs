// The Options > Mods > Horizon Essentials > Settings page: its labels, read from
// the Core snapshot ("Loading…" until the configuration has been read).
fn essentials_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::{essentials_events as ev, essentials_settings as ui, essentials_types as t};
    if mod_id != t::MOD_ID {
        return;
    }
    let game = &*game;
    ev::view(|| game.native_data_dir(t::MOD_ID), |snapshot| ui::labels(snapshot, buttons));
}
