// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn input_observe(
    game: &crate::game::Game<'_>,
    key: winit::keyboard::KeyCode,
    pressed: bool,
    repeat: bool,
) {
    crate::game::rusty_control::observe_key(game, key, pressed, repeat);
}
