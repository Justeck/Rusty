// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn clickgui_drag(game: &crate::game::Game<'_>, screen: &NativeScreenView<'_>, canceled: &mut bool) {
    if screen.id == "mod" && game.native_mod_screen() == Some("rusty") {
        *canceled |= crate::game::rusty_clickgui::drag(game, screen.mouse);
    }
}
