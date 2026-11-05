// Rusty fullbright after native settings application, Apache-2.0.
fn rusty_bright(value: (), game: &mut crate::game::Game<'_>) {
    crate::game::rusty_control::settings_applied(game);
    value
}
