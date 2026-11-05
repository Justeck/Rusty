// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_fog(value: FogData, game: &crate::game::Game<'_>) -> FogData {
    let mut value = value;
    if crate::game::rusty_control::enabled(game)
        && horizon_client_render::mesher::rusty::config().no_fog
    {
        value.environmental_start = 1.0e8;
        value.environmental_end = 1.1e8;
        value.render_distance_start = 1.0e8;
        value.render_distance_end = 1.1e8;
        value.sky_end = 1.1e8;
        value.cloud_end = 1.1e8;
    }
    value
}
