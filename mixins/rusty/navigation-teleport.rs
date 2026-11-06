// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn navigation_teleport() {
    crate::game::rusty_navigation_native::teleported();
    crate::game::rusty_awareness_native::reset();
    crate::game::rusty_builder::reset();
    crate::game::rusty_schematic_native::manual_look();
    crate::game::rusty_inventory::reset();
}
