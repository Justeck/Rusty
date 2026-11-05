// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_reset(game: &mut crate::game::Game<'_>) {
    crate::game::rusty_control::reset(game);
}
