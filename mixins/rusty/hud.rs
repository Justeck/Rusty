// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_hud(game: &crate::game::Game<'_>, batch: &mut HudBatch) {
    crate::game::rusty_hud_native::begin();
    crate::game::rusty_builder::hud(game, batch);
    crate::game::rusty_visuals::hud(game, batch);
    crate::game::rusty_navigation_native::hud(game, batch);
    crate::game::rusty_utilities::hud(game, batch);
    crate::game::rusty_awareness_native::hud(game, batch);
    let cfg = horizon_client_render::mesher::rusty::config();
    crate::game::rusty_hud_native::finish(batch, &cfg.client.hud_layout);
}
