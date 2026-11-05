// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_pad(game: &crate::game::Game<'_>, value: Option<[f32; 2]>) -> Option<[f32; 2]> {
    crate::game::rusty_camera::pad_motion(
        game,
        crate::game::rusty_control::pad_motion(game, value, game.pad_focused()),
    )
}
