// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_ui_draw(
    game: &mut crate::game::Game<'_>,
    batch: &mut HudBatch,
) -> std::ops::ControlFlow<()> {
    crate::game::rusty_ui::draw(game, batch)
}
