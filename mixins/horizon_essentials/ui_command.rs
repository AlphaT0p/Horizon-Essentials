// Horizon Essentials : la commande /essentials (crate::game::essentials_command).
// Comme la page de réglages, elle ne change rien elle-même : elle rend des lignes
// pour le chat et, s'il y a lieu, une `Action` que le Core applique.
//
//   /essentials                      état (version, profil, modules actifs)
//   /essentials list                 les modules
//   /essentials on|off|toggle <mod>  activer, désactiver, basculer
//   /essentials reset                TOUT remettre par défaut, après confirmation :
//   /essentials reset confirm          à taper dans les 15 secondes
//   /essentials reset <mod>          un seul module
//   /essentials reload               relire la configuration
//   /essentials help                 cette aide

use std::cell::Cell;
use std::time::{Duration, Instant};

use crate::game::essentials_config::Resolved;
use crate::game::essentials_events::Snapshot;
use crate::game::essentials_registry::{COUNT, MODULES, ModuleId};
use crate::game::essentials_types::{Action, VERSION};

pub const NAME: &str = "essentials";
/// La réponse à toute commande tant que la configuration est en cours de chargement.
pub const LOADING: &str = "Chargement de la configuration… réessaie dans un instant.";
/// Le délai pour confirmer une réinitialisation complète.
pub const RESET_DELAY: Duration = Duration::from_secs(15);

thread_local! {
    // Le moment de la dernière demande `/essentials reset` pas encore confirmée.
    static RESET_ASKED: Cell<Option<Instant>> = const { Cell::new(None) };
}

#[derive(Debug, Default, PartialEq)]
pub struct Reply {
    pub lines: Vec<String>,
    pub action: Option<Action>,
}

/// Les arguments si `command` (sans le `/`) est la nôtre, sinon None.
pub fn ours(command: &str) -> Option<&str> {
    let rest = command.strip_prefix(NAME)?;
    if rest.is_empty() || rest.starts_with(' ') { Some(rest.trim()) } else { None }
}

/// Exécute la commande sur l'instantané du Core. Pendant le chargement, rien
/// n'est modifié : la réponse est `LOADING`.
pub fn run(args: &str, snapshot: Option<Snapshot<'_>>) -> Reply {
    run_at(args, snapshot, Instant::now())
}

/// `run`, à l'instant `now` (les tests avancent le temps sans attendre).
pub fn run_at(args: &str, snapshot: Option<Snapshot<'_>>, now: Instant) -> Reply {
    let Some(Snapshot { resolved, profile }) = snapshot else {
        return Reply { lines: vec![LOADING.to_owned()], action: None };
    };
    let mut words = args.split_whitespace();
    let verb = words.next().unwrap_or("");
    let target = words.next();
    // Toute commande consomme une demande de réinitialisation en attente : seule
    // `reset confirm`, tapée juste après et dans le délai, la confirme.
    let asked = RESET_ASKED.take().filter(|at| now.saturating_duration_since(*at) <= RESET_DELAY);
    let mut reply = Reply::default();
    let mut say = |line: String| reply.lines.push(line);
    let module = |key: Option<&str>| key.and_then(ModuleId::from_key);
    match (verb, target) {
        ("", _) | ("status", _) => say(format!(
            "Horizon Essentials {VERSION} - profil {profile} - {}/{COUNT} modules actifs",
            resolved.active_count()
        )),
        ("list", _) => {
            for d in MODULES {
                let on = if resolved.on[d.id.index()] { "ACTIVÉ" } else { "désactivé" };
                say(format!("{} : {on} ({})", d.key, d.name));
            }
        }
        ("on" | "off" | "toggle", Some(key)) => match module(Some(key)) {
            Some(id) => {
                reply.action = Some(match verb {
                    "on" => Action::Set(id, true),
                    "off" => Action::Set(id, false),
                    _ => Action::Toggle(id),
                });
            }
            None => say(unknown(key)),
        },
        ("reset", None | Some("all")) => {
            RESET_ASKED.set(Some(now));
            say("Attention : TOUS les réglages de Horizon Essentials (tous les modules, placements".to_owned());
            say("et couleurs) seront remis par défaut. Pour confirmer, tape dans les 15 secondes :".to_owned());
            say(format!("/{NAME} reset confirm"));
        }
        ("reset", Some("confirm")) => match asked {
            Some(_) => reply.action = Some(Action::ResetAll),
            None => say(format!(
                "Aucune réinitialisation à confirmer (pas de demande, ou plus de 15 secondes). Tape d'abord /{NAME} reset."
            )),
        },
        ("reset", Some(key)) => match module(Some(key)) {
            Some(id) => reply.action = Some(Action::ResetModule(id)),
            None => say(unknown(key)),
        },
        ("reload", _) => reply.action = Some(Action::Reload),
        ("help", _) => help(&mut reply.lines),
        _ => {
            say(format!("Commande inconnue : /{NAME} {args}"));
            help(&mut reply.lines);
        }
    }
    reply
}

/// Ce que l'action a fait, pour le chat (après qu'elle a été appliquée).
pub fn outcome(action: Action, changed: bool, resolved: &Resolved) -> String {
    let state = |id: ModuleId| if resolved.on[id.index()] { "activé" } else { "désactivé" };
    match action {
        Action::Toggle(id) | Action::Set(id, _) if changed => format!("{} : {}", id.descriptor().name, state(id)),
        Action::Toggle(id) | Action::Set(id, _) => format!("{} : déjà {}", id.descriptor().name, state(id)),
        Action::ResetModule(id) => format!("{} : réglages par défaut", id.descriptor().name),
        Action::ResetAll => "Tous les réglages remis par défaut".to_owned(),
        // `reload` écrit lui-même « configuration rechargée ».
        Action::Reload => String::new(),
    }
}

fn unknown(key: &str) -> String {
    let keys: Vec<&str> = MODULES.iter().map(|d| d.key).collect();
    format!("Module inconnu : {key} (modules : {})", keys.join(", "))
}

fn help(lines: &mut Vec<String>) {
    lines.push(format!("/{NAME} [status|list|help|reload]"));
    lines.push(format!("/{NAME} on|off|toggle <module>"));
    lines.push(format!("/{NAME} reset <module> : un module ; /{NAME} reset : tout (avec confirmation)"));
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
        run(args, Some(Snapshot { resolved: &resolved, profile }))
    }

    #[test]
    fn pendant_le_chargement_rien_n_est_modifie() {
        let reply = run("off status", None);
        assert_eq!(reply.lines, [LOADING]);
        assert_eq!(reply.action, None);
    }

    #[test]
    fn reconnait_seulement_sa_commande() {
        assert_eq!(ours("essentials"), Some(""));
        assert_eq!(ours("essentials  list "), Some("list"));
        assert_eq!(ours("essentialsx"), None);
        assert_eq!(ours("gamemode creative"), None);
    }

    #[test]
    fn etat_et_liste() {
        let reply = go("", "default");
        assert_eq!(reply.lines, ["Horizon Essentials 0.1.0 - profil default - 1/1 modules actifs"]);
        assert_eq!(reply.action, None);
        assert_eq!(go("list", "default").lines, ["status : ACTIVÉ (Témoin Essentials)"]);
    }

    #[test]
    fn actions() {
        assert_eq!(go("off status", "d").action, Some(Action::Set(ModuleId::Status, false)));
        assert_eq!(go("on status", "d").action, Some(Action::Set(ModuleId::Status, true)));
        assert_eq!(go("toggle status", "d").action, Some(Action::Toggle(ModuleId::Status)));
        assert_eq!(go("reset status", "d").action, Some(Action::ResetModule(ModuleId::Status)));
        assert_eq!(go("reload", "d").action, Some(Action::Reload));
    }

    fn at(args: &str, now: Instant) -> Reply {
        let resolved = r();
        run_at(args, Some(Snapshot { resolved: &resolved, profile: "d" }), now)
    }

    #[test]
    fn reset_global_demande_une_confirmation_de_15_secondes() {
        let t0 = Instant::now();
        let ask = at("reset", t0);
        assert_eq!(ask.action, None, "la demande seule ne réinitialise rien");
        assert!(ask.lines[0].contains("TOUS les réglages"), "l'avertissement est explicite");
        assert_eq!(ask.lines[2], "/essentials reset confirm");
        assert_eq!(at("reset confirm", t0 + Duration::from_secs(14)).action, Some(Action::ResetAll));
        // La confirmation ne sert qu'une fois.
        assert_eq!(at("reset confirm", t0 + Duration::from_secs(15)).action, None);
    }

    #[test]
    fn reset_global_expire_apres_15_secondes() {
        let t0 = Instant::now();
        at("reset all", t0);
        let late = at("reset confirm", t0 + Duration::from_secs(16));
        assert_eq!(late.action, None);
        assert!(late.lines[0].starts_with("Aucune réinitialisation à confirmer"));
    }

    #[test]
    fn une_autre_commande_annule_la_demande() {
        let t0 = Instant::now();
        at("reset", t0);
        at("list", t0 + Duration::from_secs(1));
        assert_eq!(at("reset confirm", t0 + Duration::from_secs(2)).action, None);
    }

    #[test]
    fn confirmer_sans_demande_ne_fait_rien() {
        let t0 = Instant::now();
        at("list", t0);
        assert_eq!(at("reset confirm", t0).action, None);
    }

    #[test]
    fn reset_d_un_module_verifie_son_nom() {
        assert_eq!(go("reset status", "d").action, Some(Action::ResetModule(ModuleId::Status)));
        let reply = go("reset radar", "d");
        assert_eq!(reply.action, None);
        assert_eq!(reply.lines, ["Module inconnu : radar (modules : status)"]);
    }

    #[test]
    fn aucun_module_ne_porte_un_mot_reserve() {
        for d in MODULES {
            assert!(!["all", "confirm"].contains(&d.key), "« {} » masquerait /essentials reset", d.key);
        }
    }

    #[test]
    fn erreurs_sans_action() {
        let reply = go("off radar", "d");
        assert_eq!(reply.action, None);
        assert_eq!(reply.lines, ["Module inconnu : radar (modules : status)"]);
        let reply = go("danse", "d");
        assert_eq!(reply.action, None);
        assert_eq!(reply.lines.len(), 4);
        assert_eq!(go("off", "d").action, None, "sans module : aide");
    }

    #[test]
    fn compte_rendu() {
        let mut c = Config::default();
        let changed = c.apply(Action::Set(ModuleId::Status, false));
        let res = c.resolve();
        assert_eq!(outcome(Action::Set(ModuleId::Status, false), changed, &res), "Témoin Essentials : désactivé");
        assert_eq!(outcome(Action::Set(ModuleId::Status, false), false, &res), "Témoin Essentials : déjà désactivé");
    }
}
