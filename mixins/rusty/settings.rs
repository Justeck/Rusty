// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_settings(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    crate::game::rusty_ui::settings(game, mod_id, buttons);
}
