// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
use super::Game;
use horizon_client::menu::Input;
pub(crate) fn click(game: &mut Game<'_>, source: usize, hotbar: Option<usize>) -> bool {
    if !game.was_playing
        || !game.native_playing()
        || game.remote.is_none()
        || game.remote_menu().0 != 0
        || game.carried.is_some()
        || source >= 36
    {
        return false;
    }
    if let Some(target) = hotbar {
        if target >= 9 || source < 9 || game.inventory[target].is_some() {
            return false;
        }
        game.remote_menu_ops(&[(Some(source), target as i32, Input::Swap)])
    } else {
        game.remote_menu_ops(&[(Some(source), 0, Input::QuickMove)])
    }
}

/// Explicit construction may displace a full hotbar slot. The controller
/// retains both stacks and waits for actual incoming receipts for both slots.
pub(in crate::game) fn construction_swap(
    game: &mut Game<'_>,
    source: usize,
    target: usize,
) -> Option<u64> {
    if !crate::game::rusty_schematic_native::stats_reliable() {
        return None;
    }
    let dispatched = game.was_playing
        && game.native_playing()
        && game.remote.is_some()
        && game.remote_menu().0 == 0
        && game.carried.is_none()
        && (9..36).contains(&source)
        && target < 9
        && game.remote_menu_ops(&[(Some(source), target as i32, Input::Swap)]);
    if !dispatched {
        return None;
    }
    // Same ordinary read-only request used by Game::open_stats. Vanilla
    // processes it after ContainerClick in its server packet queue. The
    // incoming AwardStats ordinal is an ordering fence, never a fake slot ACK.
    game.send(horizon_protocol::packets::play::Serverbound::ClientCommand(
        horizon_protocol::packets::play::ClientCommand { action: 1 },
    ));
    Some(crate::game::rusty_schematic_native::stats_last_request())
}
