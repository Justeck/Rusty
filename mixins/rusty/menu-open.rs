// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn menu_open(value: bool, game: &mut crate::game::Game<'_>) -> bool {
    if value {
        crate::game::rusty_control::consume_menu_inputs(game);
    }
    value
}
