// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_schematic_column_change(map: &crate::chunk_map::ChunkMap, key: (i32, i32)) {
    crate::mesher::rusty_schematic_build::survey_changed(map as *const _ as usize, Some(key));
}
