// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Extend world-space gizmos before the engine's debug-visibility early returns.
fn rusty_gizmos(game: &crate::game::Game<'_>, g: &mut horizon_client::gizmos::Gizmos) {
    let (camera, yaw, pitch) =
        horizon_client_render::mesher::rusty_freecam::view(game.clock.camera_fraction() as f32)
            .unwrap_or((game.view.position, game.view.y_rot, game.view.x_rot));
    let previous = std::mem::take(g);
    *g = crate::game::rusty_visuals::append_gizmos(
        previous,
        game,
        camera,
        horizon_client::camera::view_matrix(yaw, pitch),
    );
}
