// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn menu_close(game: &mut crate::game::Game<'_>) {
    crate::game::rusty_control::consume_menu_inputs(game);
}
