// Horizon Essentials : le registre statique des modules (crate::game::essentials_registry).
//
// Ajouter un module, c'est :
//   1. écrire son fichier mod_<nom>.rs (et l'ajouter à horizon-mixin.toml et harness/lib.rs) ;
//   2. ajouter une variante à `ModuleId` et une ligne à `MODULES` ci-dessous ;
//   3. ajouter son champ à `Modules` et une branche à `Modules::get_mut` (dans ce même fichier).
// Le reste du Core, le menu, la configuration et les profils le prennent en compte seuls.

use crate::game::essentials_types::Anchor;

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
