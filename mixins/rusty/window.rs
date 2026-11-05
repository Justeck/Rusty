// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_window(
    game: &mut crate::game::Game<'_>,
    event: &winit::event::WindowEvent,
    id: winit::window::WindowId,
) -> std::ops::ControlFlow<()> {
    if game.window.as_ref().is_none_or(|window| window.id() != id) {
        return std::ops::ControlFlow::Continue(());
    }
    if crate::game::rusty_control::window_blocked(game, event) {
        return std::ops::ControlFlow::Break(());
    }
    crate::game::rusty_control::window(game, event);
    crate::game::rusty_camera::window(game, event)
}
