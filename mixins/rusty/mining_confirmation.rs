// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Observe the host's ordinary server-acknowledged state. No packets are sent.
use super::Game;
pub(in crate::game) fn settled(game: &Game<'_>, pos: [i32; 3]) -> bool {
    game.remote
        .as_ref()
        .is_some_and(|r| !r.predictions.contains_key(&pos))
}
