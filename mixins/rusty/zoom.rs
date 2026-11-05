// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_zoom(
    game: &crate::game::Game<'_>,
    zoom: &mut Option<crate::game::native_hooks::NativeZoom>,
) {
    crate::game::rusty_control::zoom(game, zoom);
}
