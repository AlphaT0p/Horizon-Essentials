// A key or a mouse button (vanilla name: key.keyboard.w, key.mouse.left).
fn essentials_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::{essentials_events as ev, essentials_types as t};
    if !game.native_on(t::MOD_ID) || !game.native_window() {
        return;
    }
    let game = &*game;
    ev::key(|| game.native_data_dir(t::MOD_ID), key, pressed);
}
