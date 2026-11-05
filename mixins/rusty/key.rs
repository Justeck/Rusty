// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    crate::game::rusty_control::key(game, key, pressed);
}
