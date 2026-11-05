// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_turn(xo: f64, yo: f64) -> std::ops::ControlFlow<()> {
    crate::game::rusty_camera::turn(xo, yo)
}
