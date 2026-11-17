// Game shutdown (once, before the final save): the last configuration change is
// written and the I/O thread is waited for (2 s at most).
// No `native_on` check: a change made before turning the mod off must be kept.
fn essentials_stopping(game: &crate::game::Game<'_>) {
    if !game.native_window() {
        return;
    }
    crate::game::essentials_events::shutdown();
}
