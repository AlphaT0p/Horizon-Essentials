// Every window frame: time passes for the Core (deferred saving) and for the
// modules. The configuration is loaded on the first frame.
fn essentials_frame(game: &mut crate::game::Game<'_>, elapsed: std::time::Duration) {
    use crate::game::{essentials_events as ev, essentials_types as t};
    if !game.native_on(t::MOD_ID) || !game.native_window() {
        return;
    }
    let game = &*game;
    ev::frame(
        || game.native_data_dir(t::MOD_ID),
        elapsed.as_secs_f32() * 1000.0,
    );
}
