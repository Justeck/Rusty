// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_weather(
    game: &crate::game::Game<'_>,
) -> std::ops::ControlFlow<Vec<horizon_client::weather::Vertex>> {
    if crate::game::rusty_control::enabled(game)
        && horizon_client_render::mesher::rusty::config().clear_weather
    {
        std::ops::ControlFlow::Break(Vec::new())
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
