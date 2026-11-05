// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_inventory_received(_value: (), game: &crate::game::Game<'_>) {
    crate::game::rusty_inventory::received(game);
}
