// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Received-state box plan plus an explicitly requested, single normal placement.
use super::{Game, agent::rusty_build_agent as aim, remote::rusty_build_actions as action};
use horizon_client::{gizmos::Gizmos, hud::HudBatch, items::ItemStack};
use horizon_client_render::mesher::{
    rusty, rusty_build_plan as plan, rusty_freecam, rusty_navigation::Scope,
};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

struct Pending {
    started: Instant,
    slot: usize,
    stack: ItemStack,
    phase: Phase,
}
enum Phase {
    Queued,
    Aimed {
        target: [i32; 3],
        tick: u64,
        yaw: f32,
        pitch: f32,
    },
    Sent {
        target: [i32; 3],
        tick: u64,
        state: u32,
        sequence: i32,
    },
}
#[derive(Default)]
struct State {
    scope: Option<Scope>,
    origin: Option<[i32; 3]>,
    settings: Option<plan::BuildSettings>,
    cells: Vec<[i32; 3]>,
    refreshed: Option<Instant>,
    geometry: Gizmos,
    shown: usize,
    matching: usize,
    missing: usize,
    occupied: usize,
    unknown: usize,
    material: Option<String>,
    pending: Option<Pending>,
    result: Option<String>,
    accepted: u32,
}
thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

fn loaded(game: &Game<'_>, at: [i32; 3]) -> bool {
    at[1] >= game.world.bottom_y()
        && at[1] < game.world.top_y()
        && game.world.chunks.contains_key(&(at[0] >> 4, at[2] >> 4))
}
fn usable(game: &Game<'_>) -> bool {
    if super::rusty_mining_native::active() { return false; }
    game.native_window()
        && game.native_on(rusty::ID)
        && game.remote.is_some()
        && game.focused
        && !rusty_freecam::active()
        && !game.spectator()
        && game.riding.is_none()
        && game
            .player
            .as_ref()
            .is_some_and(|p| !p.sleeping && !p.health.dead_or_dying())
}
fn other_owned(game: &Game<'_>) -> bool {
    super::rusty_assist::busy()
        || super::rusty_automations::busy()
        || super::rusty_inventory::busy()
        || game.agent_busy()
        || game.flight.is_some()
}
fn manual(game: &Game<'_>) -> bool {
    // No controls are synthesized/held by this tool. Any physical input after
    // aiming cancels the queued action before it can use the new crosshair.
    !game.physical_buttons.is_empty()
        || !game.physical_keys.is_empty()
        || game.left
        || game.right
        || game.settings.controller.enabled
            && (game.pad_move().is_some()
                || game.pad.state.buttons != 0
                || game
                    .pad
                    .state
                    .triggers
                    .iter()
                    .any(|v| v.abs() > game.settings.controller.trigger_threshold)
                || horizon_client::gamepad::move_vector(
                    game.pad.state.right,
                    game.settings.controller.look_deadzone,
                )
                .is_some())
}
fn material(game: &Game<'_>) -> Option<String> {
    let stack = game.held()?;
    let state = game.items().get(&stack.item)?.block?;
    Some(game.res.facts().get(state).name.to_string())
}
fn replaceable(game: &Game<'_>, at: [i32; 3]) -> bool {
    loaded(game, at) && {
        let state = game.world.block(at[0], at[1], at[2]);
        game.res.facts().is_air(state) || game.res.facts().get(state).replaceable
    }
}
fn cube(out: &mut Gizmos, at: [i32; 3], color: u32) {
    for axis in 0..3 {
        for (a, b) in [(0., 0.), (1., 0.), (0., 1.), (1., 1.)] {
            let mut from = at.map(f64::from);
            from[(axis + 1) % 3] += a;
            from[(axis + 2) % 3] += b;
            let mut to = from;
            to[axis] += 1.;
            out.line(from, to, color, 1.);
        }
    }
}
fn bounded_stack(stack: &ItemStack) -> bool {
    !stack.data().is_some_and(|data| {
        data.wire
            .patch()
            .is_none_or(|patch| patch.len() > 16 * 1024)
    })
}
pub(super) fn reset() {
    STATE.with_borrow_mut(|state| *state = State::default());
}
pub(super) fn busy() -> bool {
    STATE.with_borrow(|state| state.pending.is_some())
}
pub(super) fn anchor(game: &mut Game<'_>) -> Result<(), String> {
    let settings = rusty::config().build.clone();
    if !settings.enabled || !usable(game) {
        return Err("Enable the build preview in an active world first".into());
    }
    let scope = super::rusty_navigation_native::scope(game)?;
    let hit = game
        .pick()
        .ok_or("Look at a block face before setting the plan origin")?;
    if hit.face == [0; 3] {
        return Err("Look at an outside block face".into());
    }
    let origin = std::array::from_fn(|i| hit.pos[i] + hit.face[i]);
    if !loaded(game, origin) {
        return Err("The origin must be inside received terrain and world height".into());
    }
    let cells = plan::cells(&settings, origin)?;
    STATE.with_borrow_mut(|state| {
        *state = State {
            scope: Some(scope),
            origin: Some(origin),
            settings: Some(settings),
            cells,
            result: Some("Plan origin set; preview only".into()),
            ..Default::default()
        }
    });
    Ok(())
}
pub(super) fn place_next(game: &mut Game<'_>) -> Result<(), String> {
    if !usable(game) || !game.native_playing() || super::rusty_ui::is_open() {
        return Err("Close menus in an active world before placing".into());
    }
    let cfg = rusty::config();
    if !cfg.build.enabled || other_owned(game) || game.using.is_some() || game.hands_busy() {
        return Err("Finish the current item, movement or inventory action first".into());
    }
    let scope = super::rusty_navigation_native::scope(game)?;
    let held = game
        .held()
        .ok_or("Hold the block to place in your main hand")?;
    if !bounded_stack(held) {
        return Err("The held item's component data is unavailable or exceeds 16 KiB".into());
    }
    let stack = held.clone();
    if !game
        .items()
        .get(&stack.item)
        .is_some_and(|i| i.block.is_some() && i.consumable.is_none())
    {
        return Err("Hold a nonconsumable block item in your main hand".into());
    }
    STATE.with_borrow_mut(|state| {
        if state.pending.is_some() {
            return Err("A single placement is already pending".into());
        }
        if state.scope.as_ref() != Some(&scope) || state.origin.is_none() {
            return Err("Set the plan origin in this world first".into());
        }
        if state.settings.as_ref() != Some(&cfg.build) {
            state.cells = plan::cells(&cfg.build, state.origin.unwrap())?;
            state.settings = Some(cfg.build.clone());
        }
        state.pending = Some(Pending {
            started: Instant::now(),
            slot: game.selected,
            stack,
            phase: Phase::Queued,
        });
        state.result = Some("One placement queued; release inputs".into());
        Ok(())
    })
}

pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    let cfg = rusty::config();
    if !cfg.build.enabled || !game.native_on(rusty::ID) || game.native_player().is_none() {
        reset();
        return;
    }
    let Ok(scope) = super::rusty_navigation_native::scope(game) else {
        reset();
        return;
    };
    STATE.with_borrow_mut(|state| {
        if state.scope.as_ref().is_some_and(|old| old != &scope) { *state = State::default(); }
        if state.settings.as_ref().is_some_and(|old| old != &cfg.build) {
            state.pending = None;
            state.cells = state.origin.and_then(|at| plan::cells(&cfg.build, at).ok()).unwrap_or_default();
            state.settings = Some(cfg.build.clone());
            state.refreshed = None;
            state.result = Some("Plan settings changed; any queued action was cancelled".into());
        }
        if let Some(mut pending) = state.pending.take() {
            let stack_unchanged = game.selected == pending.slot && game.held().is_some_and(|s|
                bounded_stack(s) && s.same_item_same_components(&pending.stack) && s.count == pending.stack.count);
            let waiting_ack = matches!(pending.phase, Phase::Sent { .. });
            if !usable(game) || !game.native_playing() || super::rusty_ui::is_open()
                || other_owned(game) || game.using.is_some() || !waiting_ack && !stack_unchanged
            {
                state.result = Some("Placement cancelled: its world, item or action context changed".into());
            } else if pending.started.elapsed() > Duration::from_secs(5) {
                state.result = Some("Placement timed out; no automatic retry".into());
            } else {
                let mut keep = true;
                match pending.phase {
                    Phase::Queued => {
                        if game.was_playing && manual(game) {
                            state.result = Some("Placement cancelled: manual input took control".into());
                            keep = false;
                        } else if game.was_playing && game.use_delay == 0 {
                            let mut candidates: Vec<_> = state.cells.iter().copied().filter(|at| replaceable(game, *at)).collect();
                            let p = game.native_player().unwrap();
                            candidates.sort_by(|a, b| a[1].cmp(&b[1]).then_with(|| {
                                let distance = |at: &[i32; 3]| (f64::from(at[0]) + 0.5 - p.x).powi(2)
                                    + (f64::from(at[2]) + 0.5 - p.z).powi(2);
                                distance(a).total_cmp(&distance(b))
                            }));
                            let old = (game.yaw, game.pitch);
                            let target = candidates.into_iter().take(32).find(|at| aim::aim(game, *at).is_ok());
                            if let Some(target) = target {
                                pending.phase = Phase::Aimed { target, tick: game.ticks, yaw: game.yaw, pitch: game.pitch };
                                state.result = Some("Aimed at one planned block; waiting for normal update".into());
                            } else {
                                game.yaw = old.0; game.pitch = old.1;
                                state.result = Some("No missing planned block is reachable on received solid support".into());
                                keep = false;
                            }
                        }
                    }
                    Phase::Aimed { target, tick, yaw, pitch } => {
                        if manual(game) || game.yaw != yaw || game.pitch != pitch || !replaceable(game, target) {
                            state.result = Some("Placement cancelled: input, aim or target changed".into()); keep = false;
                        } else if game.was_playing && game.ticks > tick && game.use_delay == 0 {
                            match action::place(game, target) {
                                Ok(receipt) => {
                                    pending.phase = Phase::Sent { target, tick: game.ticks, state: receipt.state, sequence: receipt.sequence };
                                    state.result = Some("One ordinary placement requested; waiting for server state".into());
                                }
                                Err(error) => { state.result = Some(error); keep = false; }
                            }
                        }
                    }
                    Phase::Sent { target, tick, state: expected, .. } => {
                        if manual(game) {
                            state.result = Some("The request was sent; manual input ended its observation without attributing a result".into());
                            keep = false;
                        } else if !loaded(game, target) {
                            state.result = Some("Placement ended: target terrain is no longer received".into()); keep = false;
                        } else if game.ticks > tick && !action::unsettled(game, target) {
                            let confirmed = game.world.block(target[0], target[1], target[2]).0 == expected;
                            if confirmed { state.accepted = state.accepted.saturating_add(1); }
                            state.result = Some(if confirmed {
                                "Server acknowledgment and received block match the requested placement"
                            } else { "Server settled the request without the expected block; no retry" }.into());
                            state.refreshed = None; keep = false;
                        }
                    }
                }
                if keep { state.pending = Some(pending); }
            }
        }
        if state.refreshed.is_some_and(|at| at.elapsed() < Duration::from_millis(250)) { return; }
        state.refreshed = Some(Instant::now());
        state.geometry = Gizmos::default(); state.shown = 0;
        state.matching = 0; state.missing = 0; state.occupied = 0; state.unknown = 0;
        state.material = material(game);
        let mut rows = Vec::with_capacity(state.cells.len());
        let p = game.native_player().unwrap();
        for &at in &state.cells {
            let color = if !loaded(game, at) { state.unknown += 1; 0xffe0ab69 }
                else {
                    let facts = game.res.facts().get(game.world.block(at[0], at[1], at[2]));
                    if state.material.as_deref() == Some(&*facts.name) { state.matching += 1; 0xff76be91 }
                    else if replaceable(game, at) { state.missing += 1; 0xff7cbcff }
                    else { state.occupied += 1; 0xffec7777 }
                };
            let distance = (f64::from(at[0]) + 0.5 - p.x).powi(2)
                + (f64::from(at[1]) + 0.5 - p.y).powi(2)
                + (f64::from(at[2]) + 0.5 - p.z).powi(2);
            rows.push((at, color, distance));
        }
        rows.sort_by(|a, b| a.2.total_cmp(&b.2));
        for (at, color, _) in rows.into_iter().take(128) { cube(&mut state.geometry, at, color); state.shown += 1; }
    });
}
pub(super) fn append_gizmos(game: &Game<'_>, out: &mut Gizmos) {
    if !game.native_window() || !game.native_on(rusty::ID) || !rusty::config().build.enabled {
        return;
    }
    STATE.with_borrow(|state| out.lines.extend_from_slice(&state.geometry.lines));
}
pub(super) fn hud(game: &Game<'_>, batch: &mut HudBatch) {
    if !game.native_playing()
        || !game.native_on(rusty::ID)
        || game.ui.hide_gui
        || !rusty::config().build.enabled
    {
        return;
    }
    let start = batch.vertices.len();
    STATE.with_borrow(|state| {
        let mut text = if state.origin.is_some() {
            format!(
                "Plan: {} matching material / {} missing / {} occupied / {} unknown",
                state.matching, state.missing, state.occupied, state.unknown
            )
        } else {
            "Build preview: Set Origin on a block face".into()
        };
        while game.native_text_width(&text) > (batch.width - 12.).max(0.) && !text.is_empty() {
            text.pop();
        }
        let y = (batch.height - 63.).max(12.);
        game.native_fill(
            batch,
            [3., y - 2., game.native_text_width(&text) + 6., 11.],
            0x90000000,
        );
        game.native_text(batch, &text, [6., y], 0xffedddca, true);
    });
    super::rusty_hud_native::place(
        batch,
        start,
        &rusty::config().client.hud_layout,
        "build_planner",
    );
}
pub(super) fn snapshot() -> serde_json::Value {
    STATE.with_borrow(|state| {
        let phase = state.pending.as_ref().map(|p| match p.phase {
            Phase::Queued => "queued", Phase::Aimed { .. } => "aimed", Phase::Sent { .. } => "awaiting_server",
        });
        let target = state.pending.as_ref().and_then(|p| match p.phase {
            Phase::Queued => None, Phase::Aimed { target, .. } | Phase::Sent { target, .. } => Some(target),
        });
        let sequence = state.pending.as_ref().and_then(|p| match p.phase { Phase::Sent { sequence, .. } => Some(sequence), _ => None });
        serde_json::json!({"origin":state.origin,"dimension":state.scope.as_ref().map(|s|&s.dimension),
            "cells":state.cells.len(),"shown":state.shown,"matching_material":state.matching,"missing":state.missing,
            "occupied":state.occupied,"unknown":state.unknown,"material":state.material,"phase":phase,
            "target":target,"sequence":sequence,"busy":state.pending.is_some(),"accepted":state.accepted,
            "result":state.result,"depth_lines":state.geometry.lines.len()})
    })
}
