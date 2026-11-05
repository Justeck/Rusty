// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Installed in the native fishing module: observe only this client's hook.
use super::Game;
use horizon_client_render::mesher::rusty_automation::HookObservation;

pub(crate) fn own_hook(game: &Game<'_>) -> Option<HookObservation> {
    if game.remote.is_none() {
        return None;
    }
    game.hook.as_ref().map(|hook| HookObservation {
        id: hook.id,
        biting: hook.nibble > 0,
    })
}
