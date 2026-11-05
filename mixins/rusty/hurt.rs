// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_hurt(value: f64, game: &crate::game::Game<'_>) -> f64 {
    if crate::game::rusty_control::enabled(game)
        && horizon_client_render::mesher::rusty::config().no_hurt_camera
    {
        0.0
    } else {
        value
    }
}
