// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_far(value: (u32, bool)) -> (u32, bool) {
    if horizon_client_render::mesher::rusty::active()
        && !horizon_client_render::mesher::rusty::config().normal_terrain()
    {
        (0, value.1)
    } else {
        value
    }
}
