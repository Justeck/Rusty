// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_schematic_bulk_edit(world: &crate::world::World, runs: &[crate::block_edit::Run]) {
    let owner = &world.chunks as *const _ as usize;
    for run in runs {
        crate::mesher::rusty_schematic_build::survey_changed(owner, Some((run.x >> 4, run.z >> 4)));
    }
}
