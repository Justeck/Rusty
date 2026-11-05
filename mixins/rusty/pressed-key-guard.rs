// Rusty detached camera body action guard, Apache-2.0.
fn rusty_pressed_key_guard(
    game: &crate::game::Game<'_>,
    key: winit::keyboard::KeyCode,
) -> std::ops::ControlFlow<Result<()>> {
    if crate::game::rusty_camera::mapped_key(game, key) {
        std::ops::ControlFlow::Break(Ok(()))
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
