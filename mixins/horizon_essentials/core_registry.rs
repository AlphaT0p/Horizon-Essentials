// Horizon Essentials : le registre statique des modules (crate::game::essentials_registry).
//
// Ajouter un module, c'est :
//   1. écrire son fichier mod_<nom>.rs (et l'ajouter à horizon-mixin.toml et harness/lib.rs) ;
//   2. ajouter une variante à `ModuleId` et une ligne à `MODULES` ci-dessous ;
//   3. ajouter son champ à `Modules` et une branche à `Modules::get_mut` (dans ce même fichier).
// Le reste du Core, le menu, la configuration et les profils le prennent en compte seuls.

use crate::game::essentials_status::Status;
use crate::game::essentials_types::{Anchor, Module};

/// Un module connu. L'ordre des variantes est l'ordre de `MODULES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModuleId {
    Status,
}

/// La fiche d'un module : ce que le Core, l'interface et la configuration en savent.
#[derive(Clone, Copy, Debug)]
pub struct Descriptor {
    pub id: ModuleId,
    /// Son nom dans les fichiers et les commandes (`a-z`, `0-9`, `_`) : ne jamais le changer.
    pub key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub default_on: bool,
    pub has_widget: bool,
    pub default_anchor: Anchor,
}

pub const MODULES: &[Descriptor] = &[Descriptor {
    id: ModuleId::Status,
    key: "status",
    name: "Témoin Essentials",
    description: "Une petite boîte qui montre que le Core tourne, avec le profil actif.",
    default_on: true,
    has_widget: true,
    default_anchor: Anchor::TopRight,
}];

/// Le nombre de modules.
pub const COUNT: usize = MODULES.len();

impl ModuleId {
    /// Sa place dans `MODULES` (et dans les tableaux du Core).
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn descriptor(self) -> &'static Descriptor {
        &MODULES[self.index()]
    }

    pub fn key(self) -> &'static str {
        self.descriptor().key
    }

    pub fn from_key(key: &str) -> Option<ModuleId> {
        MODULES.iter().find(|d| d.key == key).map(|d| d.id)
    }

    /// Tous les modules, dans l'ordre du registre.
    pub fn all() -> impl Iterator<Item = ModuleId> {
        MODULES.iter().map(|d| d.id)
    }
}

/// L'état de chaque module, un champ par module.
#[derive(Debug, Default)]
pub struct Modules {
    pub status: Status,
}

impl Modules {
    /// Le module `id`, par son interface commune : une référence, sans allocation.
    pub fn get_mut(&mut self, id: ModuleId) -> &mut dyn Module {
        #[cfg(test)]
        CALLS.with(|c| {
            let mut counts = c.get();
            counts[id.index()] += 1;
            c.set(counts);
        });
        match id {
            ModuleId::Status => &mut self.status,
        }
    }
}

// Tests : combien de fois chaque module a été appelé sur ce thread. `get_mut` est
// la seule porte vers un module, donc ce compteur voit tous les appels.
#[cfg(test)]
thread_local! {
    static CALLS: std::cell::Cell<[u64; COUNT]> = const { std::cell::Cell::new([0; COUNT]) };
}

#[cfg(test)]
pub fn calls(id: ModuleId) -> u64 {
    CALLS.with(|c| c.get()[id.index()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_ordre_du_registre_suit_l_enum() {
        for (i, d) in MODULES.iter().enumerate() {
            assert_eq!(d.id.index(), i, "{} n'est pas à sa place dans MODULES", d.key);
        }
    }

    #[test]
    fn les_cles_sont_uniques_et_valides() {
        for (i, d) in MODULES.iter().enumerate() {
            assert!(!d.key.is_empty());
            assert!(d.key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
            assert!(MODULES[i + 1..].iter().all(|o| o.key != d.key), "clé en double : {}", d.key);
            assert_eq!(ModuleId::from_key(d.key), Some(d.id));
        }
        assert_eq!(ModuleId::from_key("inconnu"), None);
    }
}
