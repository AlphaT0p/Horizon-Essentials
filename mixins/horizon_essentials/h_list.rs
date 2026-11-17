// The Horizon Essentials entry in Options > Mods.
fn essentials_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::essentials_types as t;
    mods.push(crate::game::native_hooks::NativeMod {
        id: t::MOD_ID.into(),
        title: t::title(),
        description: t::DESCRIPTION.into(),
        keys: Vec::new(),
    });
}
