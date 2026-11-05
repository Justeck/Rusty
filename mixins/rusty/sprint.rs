// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_sprint_before_movement(game: &mut crate::game::Game<'_>) {
    crate::game::rusty_control::sprint(game);
}
