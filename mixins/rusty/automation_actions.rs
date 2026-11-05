// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Installed beside the ordinary remote client action implementations.
use super::Game;
use horizon_client::first_person::{Hand, SwingAnimation};

pub(crate) fn use_main(game: &mut Game<'_>) -> bool {
    if !game.was_playing
        || !game.native_playing()
        || game.remote.is_none()
        || game.hands_busy()
        || game.using.is_some()
        || game.use_delay > 0
    {
        return false;
    }
    if let Some(swing) = game.native_interaction(1, Hand::Main) {
        if swing {
            game.swing_hand(Hand::Main, SwingAnimation::DEFAULT);
        }
        return false;
    }
    game.remote_send_carried();
    game.use_delay = 4;
    let before = game.remote.as_ref().map(|remote| remote.sequence);
    let used = game.remote_use_item(Hand::Main);
    if used {
        game.heights.item_used(Hand::Main);
    }
    // Prediction can decline an item after its normal request was sent.
    // Observe the existing sequence, never modify or construct a packet.
    matches!((before,game.remote.as_ref().map(|remote|remote.sequence)),(Some(before),Some(after)) if before!=after)
}

pub(crate) fn swap_offhand(game: &mut Game<'_>) {
    if !game.was_playing || !game.native_playing() {
        return;
    }
    game.remote_send_carried();
    game.remote_swap_hands();
}
