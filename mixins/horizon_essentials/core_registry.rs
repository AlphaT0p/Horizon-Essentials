// Horizon Essentials: the static module registry (crate::game::essentials_registry).
//
// Adding a module means:
//   1. writing its mod_<name>.rs file (and adding it to horizon-mixin.toml and harness/lib.rs);
//   2. adding a `ModuleId` variant and a `MODULES` row below;
//   3. adding its field to `Modules` and a branch to `Modules::get_mut` (in this same file).
// The rest of the Core, the menu, the configuration and the profiles pick it up on their own.

use crate::game::essentials_status::Status;
use crate::game::essentials_types::{Anchor, Module};

/// A known module. The variant order is the order of `MODULES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModuleId {
    Status,
}

/// The menu's module categories, in tab order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// Boxes drawn on the HUD.
    Hud,
    /// Combat helpers.
    Pvp,
    /// Everything else.
    Utility,
}

impl Category {
    pub const ALL: [Category; 3] = [Category::Hud, Category::Pvp, Category::Utility];

    pub fn label(self) -> &'static str {
        match self {
            Category::Hud => "HUD",
            Category::Pvp => "PvP",
            Category::Utility => "Utility",
        }
    }
}

/// A module's descriptor: what the Core, the UI and the configuration know about it.
#[derive(Clone, Copy, Debug)]
pub struct Descriptor {
    pub id: ModuleId,
    /// Its name in files and commands (`a-z`, `0-9`, `_`): never change it.
    pub key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: Category,
    pub default_on: bool,
    pub has_widget: bool,
    pub default_anchor: Anchor,
}

impl Descriptor {
    /// Whether the menu shows it: in `category` (None: all) and with `query` in its name or
    /// description, ignoring ASCII case (an empty query matches everything).
    pub fn matches(&self, category: Option<Category>, query: &str) -> bool {
        let found = |text: &str| {
            query.is_empty()
                || text
                    .as_bytes()
                    .windows(query.len())
                    .any(|w| w.eq_ignore_ascii_case(query.as_bytes()))
        };
        category.is_none_or(|c| c == self.category) && (found(self.name) || found(self.description))
    }
}

pub const MODULES: &[Descriptor] = &[Descriptor {
    id: ModuleId::Status,
    key: "status",
    name: "Essentials Status",
    description: "A small box showing that the Core is running, with the active profile.",
    category: Category::Hud,
    default_on: true,
    has_widget: true,
    default_anchor: Anchor::TopRight,
}];

/// The number of modules.
pub const COUNT: usize = MODULES.len();

impl ModuleId {
    /// Its position in `MODULES` (and in the Core's arrays).
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

    /// All modules, in registry order.
    pub fn all() -> impl Iterator<Item = ModuleId> {
        MODULES.iter().map(|d| d.id)
    }
}

/// The state of each module, one field per module.
#[derive(Debug, Default)]
pub struct Modules {
    pub status: Status,
}

impl Modules {
    /// The module `id`, through its common interface: a reference, no allocation.
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

// Tests: how many times each module was called on this thread. `get_mut` is the
// only way into a module, so this counter sees every call.
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
    fn registry_order_follows_the_enum() {
        for (i, d) in MODULES.iter().enumerate() {
            assert_eq!(d.id.index(), i, "{} is not at its place in MODULES", d.key);
        }
    }

    #[test]
    fn keys_are_unique_and_valid() {
        for (i, d) in MODULES.iter().enumerate() {
            assert!(!d.key.is_empty());
            assert!(
                d.key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            );
            assert!(
                MODULES[i + 1..].iter().all(|o| o.key != d.key),
                "duplicate key: {}",
                d.key
            );
            assert_eq!(ModuleId::from_key(d.key), Some(d.id));
        }
        assert_eq!(ModuleId::from_key("unknown"), None);
    }

    #[test]
    fn the_menu_filter_uses_category_and_search() {
        let status = ModuleId::Status.descriptor();
        assert!(status.matches(None, ""));
        assert!(status.matches(Some(Category::Hud), ""));
        assert!(!status.matches(Some(Category::Pvp), ""));
        assert!(status.matches(None, "STATUS"), "case is ignored");
        assert!(
            status.matches(None, "active profile"),
            "the description counts"
        );
        assert!(!status.matches(None, "keystrokes"));
        assert!(
            !status.matches(Some(Category::Pvp), "status"),
            "both must match"
        );
    }

    #[test]
    fn every_category_has_a_label() {
        for c in Category::ALL {
            assert!(!c.label().is_empty());
        }
    }
}
