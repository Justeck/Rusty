// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Filter presentation levels, preserving the authoritative weather state.
fn rusty_weather_levels(value: (f32, f32), game: &crate::game::Game<'_>) -> (f32, f32) {
    if crate::game::rusty_control::enabled(game)
        && horizon_client_render::mesher::rusty::config().clear_weather
    {
        (0., 0.)
    } else {
        value
    }
}
