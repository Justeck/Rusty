// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    crate::game::rusty_ui::click(game, mod_id, button);
}
