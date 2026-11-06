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
pub(in crate::game) fn prediction_sequence(game: &Game<'_>, target: [i32; 3]) -> Option<i32> {
    game.remote.as_ref()?.predictions.get(&target).map(|p| p.0)
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

/// Exact-state dry probe for explicit schematic construction. Every coupled
/// write is validated before an ordinary block-use request can be dispatched.
pub(in crate::game) fn probe(
    game: &mut Game<'_>,
    target: [i32; 3],
    plan: &std::collections::BTreeMap<[i32; 3], u32>,
) -> Result<Vec<([i32; 3], u32)>, String> {
    if game.remote_use_targets_entity() {
        return Err("An entity crossed the placement line".into());
    }
    let hit = game.pick().ok_or("No reachable support")?;
    probe_hit(game, target, plan, &hit)
}
/// The aiming search supplies a REAL normal-reach clip_ray result here. The
/// host caches pick() for a tick, so using pick() immediately after changing
/// yaw would dry-test the preceding camera. Dispatch still waits a whole tick
/// and calls probe() above against the ordinary cached crosshair again.
pub(in crate::game) fn probe_hit(
    game: &mut Game<'_>,
    target: [i32; 3],
    plan: &std::collections::BTreeMap<[i32; 3], u32>,
    hit: &crate::game::Hit,
) -> Result<Vec<([i32; 3], u32)>, String> {
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
        return Err("Ordinary placement context is not ready".into());
    }
    let held = game.held().ok_or("No selected material")?.clone();
    if held.count == 0
        || held
            .data()
            .is_some_and(|d| d.wire.patch().is_none_or(|p| p.len() > 16 * 1024))
        || game.native_input.claimant(&held.item).is_some()
        || !game
            .items()
            .get(&held.item)
            .is_some_and(|i| i.block.is_some() && i.consumable.is_none())
        || game
            .remote_block_reacts(hit, Some(&held))
            .0
            .consumes_action()
    {
        return Err("Material or clicked support requires manual use".into());
    }
    let (destination, changed) = game
        .remote_can_place(hit, &held)
        .ok_or("Ordinary placement rejected support, shape or entity collision")?;
    if [destination.x, destination.y, destination.z] != target {
        return Err("Ordinary placement redirected the target".into());
    }
    let writes: Vec<_> = changed
        .into_iter()
        .map(|(p, s)| ([p.x, p.y, p.z], s.0))
        .collect();
    let mut stages = std::collections::BTreeMap::new();
    for &(at, written) in &writes {
        let expected = *plan
            .get(&at)
            .ok_or("A coupled write is outside the schematic")?;
        if written != expected && !progress(game, at, expected, written) {
            return Err("Ordinary placement would create a wrong block state".into());
        }
        stages.insert(at, written);
    }
    horizon_client_render::mesher::rusty_schematic_build::validate_writes(
        &stages, target, &writes,
    )?;
    for &(at, expected) in &writes {
        if unsettled(game, at)
            || at[1] < game.world.bottom_y()
            || at[1] >= game.world.top_y()
            || !game.world.chunks.contains_key(&(at[0] >> 4, at[2] >> 4))
        {
            return Err("A placement cell is unknown or unsettled".into());
        }
        let actual = game.world.block(at[0], at[1], at[2]);
        let f = game.res.facts().get(actual);
        if actual.0 != expected
            && !game.res.facts().is_air(actual)
            && !f.replaceable
            && !partial(game, at, plan[&at])
        {
            return Err("Placement would replace an occupied cell".into());
        }
    }
    Ok(writes)
}
fn progress(game: &Game<'_>, at: [i32; 3], expected: u32, written: u32) -> bool {
    let registry = &game.pack.assets.registry;
    let Some(desired) = registry.state(horizon_world::registry::BlockStateId(expected)) else {
        return false;
    };
    let Some(previous) = registry.state(game.world.block(at[0], at[1], at[2])) else {
        return false;
    };
    let Some(next) = registry.state(horizon_world::registry::BlockStateId(written)) else {
        return false;
    };
    horizon_client_render::mesher::rusty_schematic_build::safe_progress(
        &desired.name,
        &desired.properties.clone().into(),
        &previous.name,
        &previous.properties.clone().into(),
        &next.name,
        &next.properties.clone().into(),
    )
}
pub(in crate::game) fn partial(game: &Game<'_>, at: [i32; 3], expected: u32) -> bool {
    let registry = &game.pack.assets.registry;
    let Some(desired) = registry.state(horizon_world::registry::BlockStateId(expected)) else {
        return false;
    };
    let Some(actual) = registry.state(game.world.block(at[0], at[1], at[2])) else {
        return false;
    };
    // Recognize a received intermediate from a conceptual immediately preceding
    // stage; no world state is modified by this classification.
    let mut prior: std::collections::BTreeMap<String, String> = actual.properties.clone().into();
    for key in ["layers", "candles", "eggs", "pickles"] {
        if let Some(n) = prior.get(key).and_then(|v| v.parse::<u32>().ok()) {
            if n > 1 {
                prior.insert(key.into(), (n - 1).to_string());
                return horizon_client_render::mesher::rusty_schematic_build::safe_progress(
                    &desired.name,
                    &desired.properties.clone().into(),
                    &actual.name,
                    &prior,
                    &actual.name,
                    &actual.properties.clone().into(),
                );
            }
        }
    }
    horizon_client_render::mesher::rusty_schematic_build::safe_progress(
        &desired.name,
        &desired.properties.clone().into(),
        "minecraft:air",
        &Default::default(),
        &actual.name,
        &actual.properties.clone().into(),
    )
}
pub(in crate::game) fn place_exact(
    game: &mut Game<'_>,
    target: [i32; 3],
    plan: &std::collections::BTreeMap<[i32; 3], u32>,
    expected: &[([i32; 3], u32)],
) -> Result<(), String> {
    let writes = probe(game, target, plan)?;
    if !horizon_client_render::mesher::rusty_schematic_build::same_writes(expected, &writes) {
        return Err("Ordinary placement changed after aiming".into());
    }
    if game.native_interaction(1, Hand::Main).is_some() {
        return Err("A native interaction hook cancelled construction".into());
    }
    game.remote_send_carried();
    game.use_delay = 4;
    if !game.remote_use_block(Hand::Main) {
        return Err("Ordinary placement was not dispatched".into());
    }
    if writes.iter().any(|(at, _)| !unsettled(game, *at)) {
        return Err("Ordinary placement did not predict every planned write; stopped".into());
    }
    Ok(())
}
