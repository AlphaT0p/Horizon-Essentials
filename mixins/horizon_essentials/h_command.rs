// /essentials: the UI reads the Core snapshot and returns an Action, the Core
// applies it, the replies go to the chat. Other commands are left alone.
fn essentials_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    use crate::game::{essentials_command as ui, essentials_events as ev, essentials_types as t};
    let Some(args) = ui::ours(command) else {
        return;
    };
    if *ran || !game.native_on(t::MOD_ID) || !game.native_window() {
        return;
    }
    *ran = true;
    let dir = game.native_data_dir(t::MOD_ID);
    let Some(reply) = ev::view(|| dir.clone(), |snapshot| ui::run(args, snapshot)) else {
        return;
    };
    let mut lines = reply.lines;
    if let Some(action) = reply.action {
        let changed = ev::apply(|| dir.clone(), action);
        let line = ev::view(
            || dir.clone(),
            |snapshot| {
                snapshot
                    .map(|s| ui::outcome(action, changed, s.resolved))
                    .unwrap_or_default()
            },
        );
        if let Some(line) = line.filter(|l| !l.is_empty()) {
            lines.push(line);
        }
    }
    for line in lines {
        game.native_chat(&line);
    }
}
