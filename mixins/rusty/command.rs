// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    crate::game::rusty_control::command(game, command, ran);
}
