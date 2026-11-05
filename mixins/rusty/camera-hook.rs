// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_camera(partial: f32) -> std::ops::ControlFlow<Option<(glam::DVec3, f32, f32)>> {
    crate::game::rusty_camera::camera(partial)
}
