// The commands this mod runs (`native_commands`, API 35): Horizon adds them to
// the chat's command tree, so /essentials is coloured and completed instead of
// shown as unknown. Only while the mod is enabled: when it is off, `h_command`
// does not run it either.
fn essentials_commands(game: &mut crate::game::Game<'_>, commands: &mut Vec<&'static str>) {
    use crate::game::{essentials_command as ui, essentials_types as t};
    if game.native_on(t::MOD_ID) {
        commands.push(ui::NAME);
    }
}
