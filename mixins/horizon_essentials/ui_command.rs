// Horizon Essentials: the /essentials command (crate::game::essentials_command).
// Like the settings page, it changes nothing itself: it returns lines for the
// chat and, when needed, an `Action` that the Core applies.
//
//   /essentials, /essentials help    this help
//   /essentials status               version, profile, enabled modules
//   /essentials list                 the modules, their state and description
//   /essentials enable <mod>         enable a module (short form: on)
//   /essentials disable <mod>        disable a module (short form: off)
//   /essentials toggle <mod>         switch a module
//   /essentials reset                reset EVERYTHING to defaults, after confirmation:
//   /essentials reset confirm          to be typed within 15 seconds
//   /essentials reset <mod>          a single module
//   /essentials reload               read the configuration again
//
// Module names come from the registry (`MODULES`): a new module is accepted
// without changing this file.

use std::cell::Cell;
use std::time::{Duration, Instant};

use crate::game::essentials_config::Resolved;
use crate::game::essentials_events::Snapshot;
use crate::game::essentials_registry::{COUNT, MODULES, ModuleId};
use crate::game::essentials_types::{Action, VERSION};

pub const NAME: &str = "essentials";
/// The reply to any command while the configuration is loading.
pub const LOADING: &str = "Loading the configuration… try again in a moment.";
/// The delay to confirm a full reset.
pub const RESET_DELAY: Duration = Duration::from_secs(15);

thread_local! {
    // The time of the last `/essentials reset` request not yet confirmed.
    static RESET_ASKED: Cell<Option<Instant>> = const { Cell::new(None) };
}

#[derive(Debug, Default, PartialEq)]
pub struct Reply {
    pub lines: Vec<String>,
    pub action: Option<Action>,
}

/// The arguments if `command` (without the `/`) is ours, otherwise None.
pub fn ours(command: &str) -> Option<&str> {
    let rest = command.strip_prefix(NAME)?;
    if rest.is_empty() || rest.starts_with(' ') {
        Some(rest.trim())
    } else {
        None
    }
}

/// Runs the command on the Core snapshot. While loading, nothing is changed:
/// the reply is `LOADING`.
pub fn run(args: &str, snapshot: Option<Snapshot<'_>>) -> Reply {
    run_at(args, snapshot, Instant::now())
}

/// `run`, at time `now` (tests move time forward without waiting).
pub fn run_at(args: &str, snapshot: Option<Snapshot<'_>>, now: Instant) -> Reply {
    let Some(Snapshot { resolved, profile }) = snapshot else {
        return Reply {
            lines: vec![LOADING.to_owned()],
            action: None,
        };
    };
    let mut words = args.split_whitespace();
    let verb = words.next().unwrap_or("");
    let target = words.next();
    let extra = words.next();
    // Any command consumes a pending reset request: only `reset confirm`, typed
    // right after and within the delay, confirms it.
    let asked = RESET_ASKED
        .take()
        .filter(|at| now.saturating_duration_since(*at) <= RESET_DELAY);
    let mut reply = Reply::default();
    let mut say = |line: String| reply.lines.push(line);
    match (verb, target) {
        _ if extra.is_some() => say(too_many(args)),
        ("" | "help", None) => help(&mut reply.lines),
        ("status", None) => say(format!(
            "Horizon Essentials {VERSION} - profile {profile} - {}/{COUNT} modules enabled",
            resolved.active_count()
        )),
        ("list", None) => {
            for d in MODULES {
                let on = if resolved.on[d.id.index()] {
                    "ON"
                } else {
                    "off"
                };
                say(format!("{}: {on} - {}: {}", d.key, d.name, d.description));
            }
        }
        // `on` and `off` are kept as short forms of `enable` and `disable`.
        ("enable" | "on" | "disable" | "off" | "toggle", Some(key)) => {
            match ModuleId::from_key(key) {
                Some(id) => {
                    reply.action = Some(match verb {
                        "enable" | "on" => Action::Set(id, true),
                        "disable" | "off" => Action::Set(id, false),
                        _ => Action::Toggle(id),
                    });
                }
                None => say(unknown(key)),
            }
        }
        ("enable" | "on" | "disable" | "off" | "toggle", None) => say(format!(
            "Missing module name: /{NAME} {verb} <module> (modules: {})",
            module_keys()
        )),
        ("reset", None | Some("all")) => {
            RESET_ASKED.set(Some(now));
            say(
                "Warning: ALL Horizon Essentials settings (every module, position and color)"
                    .to_owned(),
            );
            say("will be reset to defaults. To confirm, type within 15 seconds:".to_owned());
            say(format!("/{NAME} reset confirm"));
        }
        ("reset", Some("confirm")) => match asked {
            Some(_) => reply.action = Some(Action::ResetAll),
            None => say(format!(
                "No reset to confirm (no request, or more than 15 seconds ago). Type /{NAME} reset first."
            )),
        },
        ("reset", Some(key)) => match ModuleId::from_key(key) {
            Some(id) => reply.action = Some(Action::ResetModule(id)),
            None => say(unknown(key)),
        },
        ("reload", None) => reply.action = Some(Action::Reload),
        ("help" | "status" | "list" | "reload", Some(_)) => say(too_many(args)),
        _ => {
            say(format!("Unknown command: /{NAME} {args}"));
            help(&mut reply.lines);
        }
    }
    reply
}

/// What the action did, for the chat (after it was applied).
pub fn outcome(action: Action, changed: bool, resolved: &Resolved) -> String {
    let state = |id: ModuleId| {
        if resolved.on[id.index()] {
            "enabled"
        } else {
            "disabled"
        }
    };
    match action {
        Action::Toggle(id) | Action::Set(id, _) if changed => {
            format!("{}: {}", id.descriptor().name, state(id))
        }
        Action::Toggle(id) | Action::Set(id, _) => {
            format!("{}: already {}", id.descriptor().name, state(id))
        }
        Action::ResetModule(id) => format!("{}: default settings", id.descriptor().name),
        Action::ResetAll => "All settings reset to defaults".to_owned(),
        // `reload` writes "configuration reloaded" itself.
        Action::Reload => String::new(),
    }
}

/// The module names the commands accept, from the registry.
fn module_keys() -> String {
    let keys: Vec<&str> = MODULES.iter().map(|d| d.key).collect();
    keys.join(", ")
}

fn unknown(key: &str) -> String {
    format!("Unknown module: {key} (modules: {})", module_keys())
}

fn too_many(args: &str) -> String {
    format!("Too many arguments: /{NAME} {args} (see /{NAME} help)")
}

fn help(lines: &mut Vec<String>) {
    lines.push(format!("/{NAME} help|status|list|reload"));
    lines.push(format!("/{NAME} enable|disable|toggle <module>"));
    lines.push(format!(
        "/{NAME} reset <module>: one module; /{NAME} reset: everything (with confirmation)"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::essentials_config::Config;

    fn r() -> Resolved {
        Config::default().resolve()
    }

    fn go(args: &str, profile: &str) -> Reply {
        let resolved = r();
        run(
            args,
            Some(Snapshot {
                resolved: &resolved,
                profile,
            }),
        )
    }

    #[test]
    fn nothing_changes_while_loading() {
        let reply = run("off status", None);
        assert_eq!(reply.lines, [LOADING]);
        assert_eq!(reply.action, None);
    }

    #[test]
    fn recognizes_only_its_own_command() {
        assert_eq!(ours("essentials"), Some(""));
        assert_eq!(ours("essentials  list "), Some("list"));
        assert_eq!(ours("essentialsx"), None);
        assert_eq!(ours("gamemode creative"), None);
    }

    #[test]
    fn the_bare_command_shows_help() {
        let reply = go("", "default");
        assert_eq!(reply.action, None);
        assert_eq!(reply.lines, go("help", "default").lines);
        assert_eq!(reply.lines.len(), 3);
        assert!(reply.lines[1].contains("enable|disable|toggle <module>"));
    }

    #[test]
    fn status_and_list() {
        let reply = go("status", "default");
        assert_eq!(
            reply.lines,
            ["Horizon Essentials 0.1.0 - profile default - 1/1 modules enabled"]
        );
        assert_eq!(reply.action, None);
        assert_eq!(
            go("list", "default").lines,
            [
                "status: ON - Essentials Status: A small box showing that the Core is running, with the active profile."
            ]
        );
    }

    #[test]
    fn list_shows_every_registry_module_with_its_state() {
        let mut config = Config::default();
        config.apply(Action::Set(ModuleId::Status, false));
        let resolved = config.resolve();
        let snapshot = Some(Snapshot {
            resolved: &resolved,
            profile: "d",
        });
        let lines = run("list", snapshot).lines;
        assert_eq!(lines.len(), MODULES.len());
        for (line, d) in lines.iter().zip(MODULES) {
            assert!(line.starts_with(d.key), "{line}");
            assert!(line.contains(d.name) && line.contains(d.description));
            let expected = if resolved.on[d.id.index()] {
                ": ON"
            } else {
                ": off"
            };
            assert!(line.contains(expected), "{line}");
        }
    }

    #[test]
    fn actions() {
        assert_eq!(
            go("enable status", "d").action,
            Some(Action::Set(ModuleId::Status, true))
        );
        assert_eq!(
            go("disable status", "d").action,
            Some(Action::Set(ModuleId::Status, false))
        );
        assert_eq!(
            go("off status", "d").action,
            Some(Action::Set(ModuleId::Status, false))
        );
        assert_eq!(
            go("on status", "d").action,
            Some(Action::Set(ModuleId::Status, true))
        );
        assert_eq!(
            go("toggle status", "d").action,
            Some(Action::Toggle(ModuleId::Status))
        );
        assert_eq!(
            go("reset status", "d").action,
            Some(Action::ResetModule(ModuleId::Status))
        );
        assert_eq!(go("reload", "d").action, Some(Action::Reload));
    }

    fn at(args: &str, now: Instant) -> Reply {
        let resolved = r();
        run_at(
            args,
            Some(Snapshot {
                resolved: &resolved,
                profile: "d",
            }),
            now,
        )
    }

    #[test]
    fn full_reset_asks_for_a_15_second_confirmation() {
        let t0 = Instant::now();
        let ask = at("reset", t0);
        assert_eq!(ask.action, None, "the request alone resets nothing");
        assert!(
            ask.lines[0].contains("ALL Horizon Essentials settings"),
            "the warning is explicit"
        );
        assert_eq!(ask.lines[2], "/essentials reset confirm");
        assert_eq!(
            at("reset confirm", t0 + Duration::from_secs(14)).action,
            Some(Action::ResetAll)
        );
        // The confirmation works only once.
        assert_eq!(
            at("reset confirm", t0 + Duration::from_secs(15)).action,
            None
        );
    }

    #[test]
    fn full_reset_expires_after_15_seconds() {
        let t0 = Instant::now();
        at("reset all", t0);
        let late = at("reset confirm", t0 + Duration::from_secs(16));
        assert_eq!(late.action, None);
        assert!(late.lines[0].starts_with("No reset to confirm"));
    }

    #[test]
    fn another_command_cancels_the_request() {
        let t0 = Instant::now();
        at("reset", t0);
        at("list", t0 + Duration::from_secs(1));
        assert_eq!(
            at("reset confirm", t0 + Duration::from_secs(2)).action,
            None
        );
    }

    #[test]
    fn confirming_without_a_request_does_nothing() {
        let t0 = Instant::now();
        at("list", t0);
        assert_eq!(at("reset confirm", t0).action, None);
    }

    #[test]
    fn module_reset_checks_its_name() {
        assert_eq!(
            go("reset status", "d").action,
            Some(Action::ResetModule(ModuleId::Status))
        );
        let reply = go("reset radar", "d");
        assert_eq!(reply.action, None);
        assert_eq!(reply.lines, ["Unknown module: radar (modules: status)"]);
    }

    #[test]
    fn no_module_uses_a_reserved_word() {
        for d in MODULES {
            assert!(
                !["all", "confirm"].contains(&d.key),
                "\"{}\" would hide /essentials reset",
                d.key
            );
        }
    }

    #[test]
    fn unknown_modules_change_nothing() {
        for args in ["enable radar", "disable radar", "toggle radar", "off radar"] {
            let reply = go(args, "d");
            assert_eq!(reply.action, None, "{args}");
            assert_eq!(reply.lines, ["Unknown module: radar (modules: status)"]);
        }
    }

    #[test]
    fn incorrect_commands_change_nothing() {
        let reply = go("dance", "d");
        assert_eq!(reply.action, None);
        assert_eq!(reply.lines[0], "Unknown command: /essentials dance");
        assert_eq!(reply.lines.len(), 4, "followed by the help");

        let reply = go("enable", "d");
        assert_eq!(reply.action, None);
        assert_eq!(
            reply.lines,
            ["Missing module name: /essentials enable <module> (modules: status)"]
        );

        for args in [
            "enable status now",
            "list all",
            "reload x",
            "reset status x",
        ] {
            let reply = go(args, "d");
            assert_eq!(reply.action, None, "{args}");
            assert_eq!(
                reply.lines,
                [format!(
                    "Too many arguments: /essentials {args} (see /essentials help)"
                )]
            );
        }
    }

    #[test]
    fn outcome_report() {
        let mut c = Config::default();
        let changed = c.apply(Action::Set(ModuleId::Status, false));
        let res = c.resolve();
        assert_eq!(
            outcome(Action::Set(ModuleId::Status, false), changed, &res),
            "Essentials Status: disabled"
        );
        assert_eq!(
            outcome(Action::Set(ModuleId::Status, false), false, &res),
            "Essentials Status: already disabled"
        );
    }
}
