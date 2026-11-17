// A click on the settings page: the UI turns it into an Action, the Core applies it.
// While the configuration is loading, a click does nothing.
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
