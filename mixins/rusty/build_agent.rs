// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Scoped access to the ordinary host's reachable-face raycast/aim algorithm.
use super::Game;

pub(in crate::game) fn aim(game: &mut Game<'_>, target: [i32; 3]) -> Result<(), String> {
    // Only full, nonreplaceable, received support cells are eligible in this
    // first explicit placement tool. Partial/replaced clicked cells can choose
    // another placement position; the normal dry placement checks that too.
    for direction in [
        [0, -1, 0],
        [0, 0, -1],
        [0, 0, 1],
        [-1, 0, 0],
        [1, 0, 0],
        [0, 1, 0],
    ] {
        let support = std::array::from_fn(|i| target[i] + direction[i]);
        if support[1] < game.world.bottom_y()
            || support[1] >= game.world.top_y()
            || !game
                .world
                .chunks
                .contains_key(&(support[0] >> 4, support[2] >> 4))
        {
            continue;
        }
        let facts = game
            .res
            .facts()
            .get(game.world.block(support[0], support[1], support[2]));
        if !facts.full_collision || facts.replaceable || facts.block_entity {
            continue;
        }
        if game.aim_block(support, Some(direction.map(|v| -v))).is_ok() {
            return Ok(());
        }
    }
    Err("No received solid support has a visible face within normal reach".into())
}
