// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_mining_look(game: &crate::game::Game<'_>, x: f64, y: f64) {
    if game.native_window() && (x != 0. || y != 0.) {
        crate::game::rusty_mining_native::manual_look();
    }
}
