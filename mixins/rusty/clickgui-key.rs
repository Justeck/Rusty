// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn clickgui_key(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    key: &str,
    repeat: bool,
    modifiers: u32,
    canceled: &mut bool,
) {
    if screen.id == "mod" && game.native_mod_screen() == Some("rusty") {
        *canceled |= crate::game::rusty_clickgui::key(game, key, true, repeat, modifiers);
    }
}
