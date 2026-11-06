// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_mining_landing(
    game: &mut crate::game::Game<'_>,
    goto: &mut Goto,
    timed_out: bool,
) -> std::ops::ControlFlow<Option<Value>> {
    // Only Rusty's one-edge mining tasks carry this ownership marker.
    // Keep the deadline and all ordinary host behavior for other tasks.
    if !timed_out
        && goto.warning.as_deref() == Some("Rusty checked mining step")
        && game.player.as_ref().is_some_and(|p| {
            horizon_client_render::mesher::rusty_mining::brake_before_landing(
                [p.position.x, p.position.y, p.position.z],
                goto.target,
                goto.near,
                p.ground.on_ground,
            )
        })
    {
        // Release ordinary forward/jump requests until landing, then let the
        // host recenter normally and acknowledge a grounded arrival.
        game.keys.remove(&winit::keyboard::KeyCode::KeyW);
        game.keys.remove(&winit::keyboard::KeyCode::Space);
        return std::ops::ControlFlow::Break(None);
    }
    std::ops::ControlFlow::Continue(())
}
