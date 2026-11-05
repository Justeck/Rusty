// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Request Horizon's native, budgeted received-terrain minimap implementation.
fn rusty_minimap(game: &crate::game::Game<'_>, minimap: &mut Option<NativeMinimap>) {
    horizon_client_render::mesher::rusty_hud_render::minimap_owned(false);
    if minimap.is_some() || !crate::game::rusty_control::enabled(game) {
        return;
    }
    let cfg = horizon_client_render::mesher::rusty::config();
    let cfg = &cfg.awareness;
    if cfg.minimap && cfg.validate().is_ok() {
        horizon_client_render::mesher::rusty_hud_render::minimap_owned(true);
        *minimap = Some(NativeMinimap {
            corner: horizon_client::extras::Corner::BottomRight,
            size: cfg.minimap_size as f32,
            blocks_per_pixel: cfg.minimap_scale,
            rotate: cfg.minimap_rotate,
            round: cfg.minimap_round,
        });
    }
}
