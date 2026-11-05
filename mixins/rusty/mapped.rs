// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_mapped(
    game: &crate::game::Game<'_>,
    key: winit::keyboard::KeyCode,
    raw: winit::keyboard::KeyCode,
    pressed: bool,
    repeat: bool,
) -> std::ops::ControlFlow<Result<()>> {
    if crate::game::rusty_control::block_mapped(game, raw, pressed, repeat)
        || crate::game::rusty_camera::mapped_key(game, key)
    {
        std::ops::ControlFlow::Break(Ok(()))
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
