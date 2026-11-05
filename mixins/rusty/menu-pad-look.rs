// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn menu_pad_look(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<()> {
    if crate::game::rusty_control::pad_look_blocked(game, game.pad_focused()) {
        std::ops::ControlFlow::Break(())
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
