// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_schematic_world_write(world: &crate::world::World, x: i32, z: i32) {
    crate::mesher::rusty_schematic_build::survey_changed(
        &world.chunks as *const _ as usize,
        Some((x >> 4, z >> 4)),
    );
}
