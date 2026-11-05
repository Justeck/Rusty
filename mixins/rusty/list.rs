// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    let mut keys = vec![[
        "menu".to_owned(),
        "Open Rusty".to_owned(),
        "key.keyboard.right.shift".to_owned(),
    ]];
    for module in horizon_client_render::mesher::rusty_catalog::MODULES {
        if let Some(id) = module.keybind_id {
            keys.push([
                id.to_owned(),
                module.name.to_owned(),
                module
                    .default_key
                    .unwrap_or("key.keyboard.unknown")
                    .to_owned(),
            ]);
        }
    }
    mods.push(crate::game::native_hooks::NativeMod {
        id:"rusty".into(),title:"Rusty".into(),
        description:"Native world, movement, inventory, combat and camera tools. Right Shift opens the menu.".into(),keys,
    });
}
