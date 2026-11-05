// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_wheel(game: &mut crate::game::Game<'_>, steps: f32, claimed: &mut bool) {
    if game.native_window() && steps.is_finite() && steps != 0. {
        crate::game::rusty_inventory::cancel_intents();
    }
    crate::game::rusty_camera::wheel(game, steps, claimed);
}
