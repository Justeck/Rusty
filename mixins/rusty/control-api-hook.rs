// Rusty optional native control integration, Apache-2.0.
fn rusty_control_api(
    game: &mut crate::game::Game<'_>,
    line: &str,
) -> std::ops::ControlFlow<Result<Option<Value>>> {
    // The host's repeat route passes its translated mapping as `raw`.
    // Consume repeats here while the original physical key is still known.
    let mut words = line.split_whitespace();
    if let (Some("key"), Some(name), Some("repeat")) = (words.next(), words.next(), words.next())
        && let Some(raw) = key_code(name)
        && crate::game::rusty_control::blocked_repeat(game, raw)
    {
        return std::ops::ControlFlow::Break(Ok(Some(
            serde_json::json!({"ok":true,"consumed":true}),
        )));
    }
    crate::game::rusty_control::control(game, line)
}
