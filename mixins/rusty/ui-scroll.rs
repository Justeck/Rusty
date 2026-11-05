// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_ui_scroll(
    game: &mut crate::game::Game<'_>,
    amount: f32,
) -> std::ops::ControlFlow<Result<()>> {
    crate::game::rusty_ui::scroll(game, amount)
}
