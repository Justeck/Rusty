// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_schematic_prediction_settlement(game: &crate::game::Game<'_>, sequence: i32) {
    if let Some(remote) = &game.remote {
        for (pos, (pending, _)) in &remote.predictions {
            if *pending <= sequence {
                horizon_client_render::mesher::rusty_schematic_build::survey_changed(
                    &game.world.chunks as *const _ as usize,
                    Some((pos[0] >> 4, pos[2] >> 4)),
                );
            }
        }
    }
}
