// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_inventory_mark(slot: usize) {
    crate::game::rusty_inventory::mark(slot);
    crate::game::rusty_schematic_native::inventory_mark(slot);
}
