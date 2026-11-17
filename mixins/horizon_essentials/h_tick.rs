// Every window tick (not the integrated server's): the modules, then at most one
// Core message in the chat, when the player is in a world.
fn essentials_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::{essentials_events as ev, essentials_types as t};
    if !game.native_on(t::MOD_ID) || !game.native_window() {
        return;
    }
    let in_world = game.native_player().is_some();
    let message = {
        let game = &*game;
        ev::tick(|| game.native_data_dir(t::MOD_ID), in_world)
    };
    if let Some(message) = message {
        game.native_chat(&message);
    }
}
