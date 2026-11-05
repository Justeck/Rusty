// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn clickgui_scroll(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    scroll: [f64; 2],
    canceled: &mut bool,
) {
    if screen.id == "mod" && game.native_mod_screen() == Some("rusty") {
        *canceled |= crate::game::rusty_clickgui::scroll(game, screen.mouse, scroll[1] as f32);
    }
}
