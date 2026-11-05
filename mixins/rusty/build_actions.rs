// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// A single ordinary main-hand block use; no generic-item/offhand fallback.
use super::Game;
use horizon_client::{
    first_person::{Hand, SwingAnimation},
    items::ItemStack,
};

pub(in crate::game) struct Receipt {
    pub state: u32,
    pub sequence: i32,
}

pub(in crate::game) fn unsettled(game: &Game<'_>, target: [i32; 3]) -> bool {
    game.remote
        .as_ref()
        .is_some_and(|r| r.predictions.contains_key(&target))
}

/// This dry native placement happens only for the user's explicit action.
/// DryLevel owns writes/RNG/events; the host's outer with_level also flushes
/// previously queued ordinary sounds, so it is not a passive preview method.
pub(in crate::game) fn place(game: &mut Game<'_>, target: [i32; 3]) -> Result<Receipt, String> {
    if !game.was_playing
        || !game.native_playing()
        || game.remote.is_none()
        || game.hands_busy()
        || game.using.is_some()
        || game.use_delay > 0
        || !game.may_build()
        || game.spectator()
        || game.remote.as_ref().is_some_and(|r| r.is_destroying)
    {
        return Err("Normal placement is not ready".into());
    }
    if game.remote_use_targets_entity() {
        return Err("An entity is in front of the intended support".into());
    }
    if let Some(swing) = game.native_interaction(1, Hand::Main) {
        if swing {
            game.swing_hand(Hand::Main, SwingAnimation::DEFAULT);
        }
        return Err("A native interaction hook cancelled placement".into());
    }
    let hit = game
        .pick()
        .ok_or("The intended support is no longer under the crosshair")?;
    let destination = std::array::from_fn(|i| hit.pos[i] + hit.face[i]);
    if destination != target || hit.face == [0; 3] {
        return Err("The current crosshair no longer points at the intended placement face".into());
    }
    let held = game.held().ok_or("Hold a block in your main hand")?;
    if held.data().is_some_and(|data| {
        data.wire
            .patch()
            .is_none_or(|patch| patch.len() > 16 * 1024)
    }) {
        return Err("The held item's component data is unavailable or exceeds 16 KiB".into());
    }
    let held: ItemStack = held.clone();
    if game.native_input.claimant(&held.item).is_some() {
        return Err("That item is owned by another native mod; use it manually".into());
    }
    if !game
        .items()
        .get(&held.item)
        .is_some_and(|info| info.block.is_some() && info.consumable.is_none())
    {
        return Err("Hold a nonconsumable block item in your main hand".into());
    }
    if game
        .remote_block_reacts(&hit, Some(&held))
        .0
        .consumes_action()
    {
        return Err("That support reacts to use; choose a plain solid support".into());
    }
    let placed = game
        .remote_can_place(&hit, &held)
        .ok_or("Native placement rejected the block shape, support or entity collision")?;
    if [placed.0.x, placed.0.y, placed.0.z] != target || placed.1.len() != 1 {
        return Err("This tool places exactly one planned block; multi-block or redirected placement is unsupported".into());
    }
    let (at, state) = placed.1[0];
    if [at.x, at.y, at.z] != target || unsettled(game, target) {
        return Err("The target has another pending placement or a redirected update".into());
    }
    game.remote_send_carried();
    game.use_delay = 4;
    // This calls exactly the same native block-use, prediction, sound and
    // inventory path as normal use. No packet or sequence is constructed here.
    if !game.remote_use_block(Hand::Main) {
        return Err("The ordinary block use was not dispatched".into());
    }
    let remote = game
        .remote
        .as_ref()
        .ok_or("Connection ended during placement")?;
    let sequence = remote
        .predictions
        .get(&target)
        .map(|(sequence, _)| *sequence)
        .ok_or("The ordinary action did not produce a placement prediction")?;
    Ok(Receipt {
        state: state.0,
        sequence,
    })
}
