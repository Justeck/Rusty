// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_ui_mouse(
    game: &mut crate::game::Game<'_>,
    pressed: bool,
    right: bool,
) -> std::ops::ControlFlow<Result<()>> {
    crate::game::rusty_ui::mouse(game, pressed, right)
}
