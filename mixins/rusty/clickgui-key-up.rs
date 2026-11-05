// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn clickgui_key_up(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    key: &str,
    modifiers: u32,
    canceled: &mut bool,
) {
    if screen.id == "mod" && game.native_mod_screen() == Some("rusty") {
        *canceled |= crate::game::rusty_clickgui::key(game, key, false, false, modifiers);
    }
}
