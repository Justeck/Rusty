// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_ui_key(
    game: &mut crate::game::Game<'_>,
    key: winit::keyboard::KeyCode,
    text: Option<&str>,
) -> std::ops::ControlFlow<Result<()>> {
    crate::game::rusty_ui::key(game, key, text)
}
