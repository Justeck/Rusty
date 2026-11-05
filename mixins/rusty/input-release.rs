// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn input_release(game: &crate::game::Game<'_>, event: &winit::event::WindowEvent) {
    crate::game::rusty_control::observe_release(game, event);
}
