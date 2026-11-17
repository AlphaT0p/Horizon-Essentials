// A click on the settings page: the UI turns it into an Action, the Core applies it.
// While the configuration is loading, or the mod is disabled, a click does nothing.
fn essentials_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::{essentials_settings as ui, essentials_types as t};
    if mod_id != t::MOD_ID {
        return;
    }
    let game = &*game;
    let on = game.native_on(t::MOD_ID);
    ui::press(on, || game.native_data_dir(t::MOD_ID), button);
}
