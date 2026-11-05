// Rusty textured navigation frames, Copyright 2026 Justeck, Apache-2.0.
// SDK359: preserve the engine's static cache snapshots, keys and statistics.
fn rusty_terrain(
    value: (
        Vec<Vertex>,
        Arc<[StaticSectionSnapshot]>,
        Vec<[i32; 3]>,
        StaticCacheFrameStats,
    ),
    game: &crate::game::Game<'_>,
    camera: glam::DVec3,
) -> (
    Vec<Vertex>,
    Arc<[StaticSectionSnapshot]>,
    Vec<[i32; 3]>,
    StaticCacheFrameStats,
) {
    let (geometry, snapshots, keys, stats) = value;
    (
        crate::game::rusty_visuals::append_terrain(geometry, game, camera.to_array()),
        snapshots,
        keys,
        stats,
    )
}
