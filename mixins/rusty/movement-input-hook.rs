// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_assistance_movement(
    game: &crate::game::Game<'_>,
    forward: &mut f32,
    strafe: &mut f32,
    jump: &mut bool,
    sneak: &mut bool,
) {
    crate::game::rusty_assist::movement(game, forward, strafe, jump, sneak);
}
