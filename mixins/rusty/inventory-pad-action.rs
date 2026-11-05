// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn inventory_pad_action(game: &crate::game::Game<'_>, action: &str, down: bool) {
    crate::game::rusty_inventory::action_input(game, action, down);
}
