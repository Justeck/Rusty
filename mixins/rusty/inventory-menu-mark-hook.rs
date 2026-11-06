// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_inventory_menu_mark(container: i32, slot: i32) {
    crate::game::rusty_inventory::mark_menu(container, slot);
    crate::game::rusty_schematic_native::inventory_menu_mark(container, slot);
}
