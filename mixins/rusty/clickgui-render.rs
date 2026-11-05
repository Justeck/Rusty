// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn clickgui_render(
    game: &crate::game::Game<'_>,
    screen: &NativeScreenView<'_>,
    batch: &mut HudBatch,
) {
    if screen.id == "mod" && game.native_mod_screen() == Some("rusty") {
        crate::game::rusty_clickgui::draw(game, batch);
    }
}
