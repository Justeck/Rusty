// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_mouse(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<Result<()>> {
    if crate::game::rusty_camera::mouse(game) {
        std::ops::ControlFlow::Break(Ok(()))
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
