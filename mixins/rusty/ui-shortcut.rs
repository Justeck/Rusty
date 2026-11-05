// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_ui_shortcut(
    game: &mut crate::game::Game<'_>,
    shortcut: horizon_client::clipboard::EditShortcut,
) -> std::ops::ControlFlow<bool> {
    crate::game::rusty_ui::shortcut(game, shortcut)
}
