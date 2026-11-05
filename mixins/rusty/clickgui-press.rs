// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn clickgui_press(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    if screen.id == "mod" && game.native_mod_screen() == Some("rusty") {
        *canceled |= crate::game::rusty_clickgui::mouse(game, screen.mouse, button, true);
    }
}
