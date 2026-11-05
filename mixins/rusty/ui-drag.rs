// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_ui_drag(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<Result<()>> {
    crate::game::rusty_ui::drag(game)
}
