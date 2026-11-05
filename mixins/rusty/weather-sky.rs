// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_weather_sky() -> std::ops::ControlFlow<()> {
    if crate::mesher::rusty::visual_enabled() && crate::mesher::rusty::config().clear_weather {
        std::ops::ControlFlow::Break(())
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
