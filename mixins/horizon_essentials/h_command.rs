// /essentials : l'interface lit l'état et rend une Action, le Core l'applique,
// les réponses vont dans le chat. Les autres commandes ne sont pas touchées.
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
    let Some(reply) = ev::view(|| dir.clone(), |resolved, profile| ui::run(args, resolved, profile)) else {
        return;
    };
    let mut lines = reply.lines;
    if let Some(action) = reply.action {
        let changed = ev::apply(|| dir.clone(), action);
        if let Some(line) = ev::view(|| dir.clone(), |resolved, _| ui::outcome(action, changed, resolved)) {
            if !line.is_empty() {
                lines.push(line);
            }
        }
    }
    for line in lines {
        game.native_chat(&line);
    }
}
