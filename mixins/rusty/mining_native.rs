// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Transient automation over received blocks, ordinary host movement and mining.
use super::{
    Game, agent::rusty_mining_agent as host, remote::rusty_mining_confirmation as confirmation,
};
use core::{Action, Grid, Node, Search, SearchResult, Settings};
use horizon_client::blocks::Fluid;
use horizon_client_render::mesher::{
    rusty, rusty_freecam, rusty_mining as core, rusty_navigation::Scope,
};
use horizon_world::registry::BlockStateId;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    sync::mpsc::{Receiver, TryRecvError},
    time::{Duration, Instant},
};

enum Phase {
    Scan {
        grid: Grid,
        index: usize,
        start: Node,
        cache: HashMap<BlockStateId, core::Cell>,
    },
    Prepare {
        grid: Grid,
        start: Node,
    },
    Search(Search),
    Act,
}
struct Pending {
    action: Action,
    receiver: Receiver<String>,
    received: bool,
    started: Instant,
    slot: Option<usize>,
}
struct Run {
    scope: Scope,
    settings: Settings,
    health: f32,
    count: u32,
    collect_until: u64,
    phase: Phase,
    actions: VecDeque<Action>,
    stand: Node,
    pending: Option<Pending>,
    previous_slot: usize,
    owned_slot: Option<usize>,
    message: String,
    started: Instant,
}
#[derive(Default)]
struct State {
    run: Option<Run>,
    result: Option<String>,
    count: u32,
}
thread_local! {static STATE:RefCell<State>=RefCell::new(State::default());static LOOK:Cell<bool>=const{Cell::new(false)};static ADVANCED:Cell<Option<u64>>=const{Cell::new(None)};}

pub(super) fn active() -> bool {
    STATE.with_borrow(|s| s.run.is_some())
}
pub(super) fn manual_look() {
    LOOK.set(true);
}
fn player_node(game: &Game<'_>) -> Result<Node, String> {
    let p = game.player.as_ref().ok_or("No player")?.position;
    Ok([p.x.floor() as i32, p.y.round() as i32, p.z.floor() as i32])
}
fn loaded(game: &Game<'_>, p: Node) -> bool {
    p[1] >= game.world.bottom_y()
        && p[1] < game.world.top_y()
        && game.world.chunks.contains_key(&(p[0] >> 4, p[2] >> 4))
}
fn suitable(game: &Game<'_>, state: BlockStateId, settings: &Settings) -> Option<(usize, u16)> {
    let facts = game.res.facts();
    let f = facts.get(state);
    let name = f.name.as_ref();
    if !f.hardness.is_finite() || f.hardness < 0. {
        return None;
    }
    game.inventory
        .iter()
        .take(9)
        .enumerate()
        .filter_map(|(slot, stack)| {
            let stack = stack.as_ref()?;
            if stack.count == 0
                || super::rusty_assist::remaining(game, stack)?
                    .is_some_and(|v| v <= settings.min_durability)
            {
                return None;
            }
            if f.requires_tool && !game.items().correct_for_drops(Some(stack), name) {
                return None;
            }
            let speed = game.items().mining_speed(Some(stack), name);
            if !speed.is_finite() || speed <= 0. {
                return None;
            }
            let ticks = (f.hardness * 30. / speed).ceil().max(1.);
            if ticks > 400. {
                return None;
            }
            Some((slot, ticks as u16))
        })
        .min_by_key(|(slot, ticks)| (*ticks, *slot != game.selected))
}
fn classify(game: &Game<'_>, p: Node, settings: &Settings) -> core::Cell {
    if !loaded(game, p) || !confirmation::settled(game, p) {
        return core::Cell::unknown();
    }
    let facts = game.res.facts();
    let state = game.world.block(p[0], p[1], p[2]);
    let f = facts.get(state);
    if facts.is_air(state) {
        return core::Cell::air();
    }
    let id = f.name.as_ref();
    if f.fluid != Fluid::None
        || f.waterlogged
        || [
            "lava",
            "fire",
            "magma",
            "cactus",
            "campfire",
            "sweet_berry",
            "powder_snow",
            "cobweb",
            "portal",
            "wither_rose",
            "pointed_dripstone",
        ]
        .iter()
        .any(|s| id.contains(s))
    {
        return core::Cell::fluid();
    }
    if id.ends_with(":sand")
        || id.ends_with("_sand")
        || id.ends_with(":gravel")
        || id.contains("anvil")
        || id.ends_with("concrete_powder")
        || f.classes.iter().any(|s| s.contains("FallingBlock"))
    {
        return core::Cell::falling();
    }
    if !f.full_collision {
        return core::Cell::fluid();
    } // Partial shapes are deliberately not navigated.
    let terrain = matches!(
        id,
        "minecraft:stone"
            | "minecraft:deepslate"
            | "minecraft:cobbled_deepslate"
            | "minecraft:netherrack"
            | "minecraft:tuff"
            | "minecraft:calcite"
            | "minecraft:granite"
            | "minecraft:diorite"
            | "minecraft:andesite"
            | "minecraft:dirt"
            | "minecraft:grass_block"
            | "minecraft:coarse_dirt"
            | "minecraft:rooted_dirt"
            | "minecraft:end_stone"
            | "minecraft:basalt"
            | "minecraft:blackstone"
    );
    if !f.block_entity && (terrain || settings.matches(id)) {
        if let Some((_, ticks)) = suitable(game, state, settings) {
            return if settings.matches(id) {
                core::Cell::resource(ticks)
            } else {
                core::Cell::terrain(ticks)
            };
        }
    }
    core::Cell::wall()
}
fn raw_air(game: &Game<'_>, p: Node) -> bool {
    loaded(game, p) && game.res.facts().is_air(game.world.block(p[0], p[1], p[2]))
}
fn snapshot(game: &Game<'_>, settings: &Settings) -> Result<(Grid, Node), String> {
    let start = player_node(game)?;
    let r = settings.radius as i32;
    let low = game.world.bottom_y();
    let high = game.world.top_y();
    Ok((
        Grid::new(
            [start[0] - r, low, start[2] - r],
            [
                (r * 2 + 1) as usize,
                (high - low) as usize,
                (r * 2 + 1) as usize,
            ],
        )?,
        start,
    ))
}
fn local_grid(game: &Game<'_>, p: Node, q: Node, settings: &Settings) -> Result<Grid, String> {
    let origin = [p[0].min(q[0]) - 2, p[1].min(q[1]) - 2, p[2].min(q[2]) - 2];
    let shape = [
        (p[0] - q[0]).unsigned_abs() as usize + 5,
        (p[1] - q[1]).unsigned_abs() as usize + 6,
        (p[2] - q[2]).unsigned_abs() as usize + 5,
    ];
    let mut grid = Grid::new(origin, shape)?;
    for i in 0..grid.len() {
        let n = grid.position(i);
        grid.set(n, classify(game, n, settings));
    }
    Ok(grid)
}
fn context(game: &Game<'_>, run: &Run, manual: bool) -> core::Context {
    let p = game.player.as_ref();
    core::Context {
        manual: manual || super::rusty_waypoint_walk::manual(game),
        damaged: p.is_none_or(|p| p.health.health() < run.health),
        same_world: super::rusty_navigation_native::scope(game).ok().as_ref() == Some(&run.scope),
        health: p.map_or(0., |p| p.health.health()),
        food: p.map_or(0, |p| p.health.food.food.max(0) as u32),
        inventory_space: game.inventory.iter().take(36).any(Option::is_none),
        focused: game.focused,
        playing: game.native_playing()
            && game.native_on(rusty::ID)
            && super::rusty_waypoint_walk::available(game)
            && !rusty_freecam::active()
            && game.remote.is_some(),
    }
}
fn release(game: &mut Game<'_>, run: &Run) {
    host::cancel(game);
    if run.owned_slot == Some(game.selected) && game.selected != run.previous_slot {
        game.selected = run.previous_slot;
    }
}
fn stop(game: &mut Game<'_>, run: Run, reason: String) {
    release(game, &run);
    STATE.with_borrow_mut(|s| {
        s.count = run.count;
        s.result = Some(reason.clone());
    });
    game.native_chat(&format!(
        "Auto Mine: {reason} ({} resource blocks)",
        run.count
    ));
}
pub(super) fn reset(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    let run = STATE.with_borrow_mut(|s| s.run.take());
    if let Some(run) = run {
        stop(game, run, "Stopped".into());
    }
}
pub(super) fn toggle(game: &mut Game<'_>) -> Result<(), String> {
    if active() {
        reset(game);
        return Ok(());
    }
    if !host::idle(game)
        || super::rusty_assist::busy()
        || super::rusty_automations::busy()
        || super::rusty_builder::busy()
        || super::rusty_inventory::busy()
    {
        return Err("Finish other automated actions first".into());
    }
    let settings = rusty::config().mining.clone();
    settings.validate()?;
    let scope = super::rusty_navigation_native::scope(game)?;
    let (grid, start) = snapshot(game, &settings)?;
    let run = Run {
        scope,
        settings,
        health: game.player.as_ref().map_or(0., |p| p.health.health()),
        count: 0,
        collect_until: 0,
        phase: Phase::Scan {
            grid,
            index: 0,
            start,
            cache: HashMap::new(),
        },
        actions: VecDeque::new(),
        stand: start,
        pending: None,
        previous_slot: game.selected,
        owned_slot: None,
        message: "Scanning received terrain".into(),
        started: Instant::now(),
    };
    if let Some(reason) = core::guard(&run.settings, &context(game, &run, false)) {
        return Err(reason.into());
    }
    if game.using.is_some() || game.left || game.right {
        return Err("Release attack/use before starting".into());
    }
    LOOK.set(false);
    ADVANCED.set(None);
    STATE.with_borrow_mut(|s| {
        s.run = Some(run);
        s.result = None;
        s.count = 0;
    });
    Ok(())
}
fn advance(game: &mut Game<'_>, run: &mut Run) -> Result<bool, String> {
    // Every hook invocation guards before touching a task; bounded planning work runs once per tick.
    if run.started.elapsed() > Duration::from_secs(1800) {
        return Err("Session time limit reached".into());
    }
    if run.settings != rusty::config().mining {
        return Err("Auto Mine settings changed; start again".into());
    }
    if let Some(reason) = core::guard(&run.settings, &context(game, run, LOOK.replace(false))) {
        return Err(reason.into());
    }
    run.health = game.player.as_ref().map_or(0., |p| p.health.health());
    if game.using.is_some()
        || !host::healthy(game)
        || host::current(game).is_some_and(|n| n != host::NAME)
    {
        return Err("Another action took over or movement stalled".into());
    }
    if !matches!(
        run.pending.as_ref().map(|p| &p.action),
        Some(Action::Walk(_))
    ) {
        let player = game.player.as_ref().ok_or("Player disappeared")?;
        if player_node(game)? != run.stand || !player.ground.on_ground {
            return Err("Player moved away from the checked standing position".into());
        }
        let mut grid = local_grid(game, run.stand, run.stand, &run.settings)?;
        if let Some(Pending {
            action: Action::Mine(pos, _),
            ..
        }) = run.pending.as_ref()
        {
            if raw_air(game, *pos) && !confirmation::settled(game, *pos) {
                grid.set(*pos, core::Cell::wall());
            }
        }

        if !grid.safe_stand(run.stand) {
            return Err("Standing terrain became unsafe".into());
        }
    }
    if run.owned_slot.is_some_and(|slot| slot != game.selected) {
        return Err("Hotbar selection changed".into());
    }
    if run.count >= run.settings.goal && run.pending.is_none() && run.actions.is_empty() {
        return Ok(game.ticks >= run.collect_until);
    }
    if let Some(pending) = run.pending.as_mut() {
        match pending.action {
            Action::Walk(to) => {
                let grid = local_grid(game, run.stand, to, &run.settings)?;
                if !grid.can_walk(run.stand, to) {
                    return Err("Walking route changed or became unsafe".into());
                }
            }
            Action::Mine(pos, _) => {
                if !raw_air(game, pos) {
                    if game.pick_entity_hit().is_some() {
                        return Err("An entity crossed the mining line".into());
                    }
                    let grid = local_grid(game, run.stand, pos, &run.settings)?;
                    if !grid.can_mine(pos, run.settings.tunnels) {
                        return Err("Mining face changed or became unsafe".into());
                    }
                    let block = game.world.block(pos[0], pos[1], pos[2]);
                    if suitable(game, block, &run.settings)
                        .is_none_or(|(slot, _)| Some(slot) != pending.slot)
                    {
                        return Err("Tool is unsuitable or below its durability limit".into());
                    }
                }
            }
        }
        if !pending.received {
            match pending.receiver.try_recv() {
                Ok(reply) => {
                    let v: serde_json::Value = serde_json::from_str(&reply)
                        .map_err(|_| "Task returned an invalid reply")?;
                    if let Some(error) = v["error"].as_str() {
                        return Err(error.chars().take(200).collect());
                    }
                    pending.received = true;
                }
                Err(TryRecvError::Disconnected) => {
                    return Err("Normal input task ended unexpectedly".into());
                }
                Err(TryRecvError::Empty) => return Ok(false),
            }
        }
        let mut collection = None;
        match pending.action {
            Action::Mine(pos, resource) => {
                if !confirmation::settled(game, pos) {
                    if pending.started.elapsed() > Duration::from_secs(35) {
                        return Err("Server confirmation timed out".into());
                    }
                    return Ok(false);
                }
                if !raw_air(game, pos) {
                    return Err("Server did not confirm an empty mined block".into());
                }
                if resource {
                    run.count += 1;
                    let grid = local_grid(game, run.stand, pos, &run.settings)?;
                    collection = grid.collection_step(run.stand, pos);
                    run.collect_until = game.ticks + 10;
                }
            }
            Action::Walk(to) => {
                let p = game.player.as_ref().ok_or("Player disappeared")?;
                if !p.ground.on_ground || (p.position.y - f64::from(to[1])).abs() > 0.3 {
                    if pending.started.elapsed() > Duration::from_secs(5) {
                        return Err("Player did not settle on the checked ground".into());
                    }
                    return Ok(false);
                }
                run.stand = to;
                if run.count >= run.settings.goal {
                    run.collect_until = game.ticks + 10;
                }
            }
        }
        run.pending = None;
        run.actions.pop_front();
        if core::collect_after(&mut run.actions, collection, run.count >= run.settings.goal) {
            run.message = "Collecting settled resource drops".into();
        }
        if run.count >= run.settings.goal && run.actions.is_empty() {
            return Ok(false);
        }
    }
    if ADVANCED.replace(Some(game.ticks)) == Some(game.ticks) {
        return Ok(false);
    }
    let phase = std::mem::replace(&mut run.phase, Phase::Act);
    match phase {
        Phase::Scan {
            mut grid,
            mut index,
            start,
            mut cache,
        } => {
            let end = (index + 8192).min(grid.len());
            while index < end {
                let p = grid.position(index);
                let c = if loaded(game, p) && confirmation::settled(game, p) {
                    let state = game.world.block(p[0], p[1], p[2]);
                    *cache
                        .entry(state)
                        .or_insert_with(|| classify(game, p, &run.settings))
                } else {
                    core::Cell::unknown()
                };
                grid.set(p, c);
                index += 1;
            }
            run.phase = if index == grid.len() {
                Phase::Prepare { grid, start }
            } else {
                Phase::Scan {
                    grid,
                    index,
                    start,
                    cache,
                }
            };
        }
        Phase::Prepare { mut grid, start } => {
            run.message = "Planning a safe route".into();
            run.phase = if grid.prepare(32768) {
                Phase::Search(Search::new(grid, start, run.settings.tunnels, 32768)?)
            } else {
                Phase::Prepare { grid, start }
            };
        }
        Phase::Search(mut search) => match search.advance(256) {
            SearchResult::Working => run.phase = Phase::Search(search),
            SearchResult::Found(plan) => {
                run.message = format!(
                    "Mining {} — estimated route {:.1}s",
                    core::RESOURCES[usize::from(run.settings.resource)],
                    plan.estimated_ticks as f32 / 20.
                );
                run.actions = plan.actions.into();
            }
            SearchResult::NoPath => {
                return Err("No safely reachable resource in the received search area".into());
            }
            SearchResult::Limited => {
                return Err("Route search limit reached; no partial route was executed".into());
            }
        },
        Phase::Act => {
            if let Some(action) = run.actions.front().cloned() {
                let receiver = match action {
                    Action::Walk(to) => {
                        let grid = local_grid(game, run.stand, to, &run.settings)?;
                        if !grid.can_walk(run.stand, to) {
                            return Err("Next step is no longer safe".into());
                        }
                        host::walk(game, run.stand, to)?
                    }
                    Action::Mine(pos, _) => {
                        let p = player_node(game)?;
                        if p[0] == pos[0] && p[2] == pos[2] {
                            return Err(
                                "Refusing to mine directly above or beneath the player".into()
                            );
                        }
                        let grid = local_grid(game, run.stand, pos, &run.settings)?;
                        if !grid.can_mine(pos, run.settings.tunnels) {
                            return Err("Next mining face is no longer safe".into());
                        }
                        let block = game.world.block(pos[0], pos[1], pos[2]);
                        let (slot, ticks) = suitable(game, block, &run.settings)
                            .ok_or("No suitable safe hotbar tool")?;
                        game.selected = slot;
                        run.owned_slot = Some(slot);
                        host::mine(game, pos, f64::from(ticks) / 20. + 5.)?
                    }
                };
                let slot = if matches!(action, Action::Mine(..)) {
                    Some(game.selected)
                } else {
                    None
                };
                run.pending = Some(Pending {
                    action,
                    receiver,
                    received: false,
                    started: Instant::now(),
                    slot,
                });
            } else {
                let (grid, start) = snapshot(game, &run.settings)?;
                run.stand = start;
                run.phase = Phase::Scan {
                    grid,
                    index: 0,
                    start,
                    cache: HashMap::new(),
                };
                run.message = "Refreshing received terrain".into();
            }
        }
    }
    Ok(false)
}
pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    let Some(mut run) = STATE.with_borrow_mut(|s| s.run.take()) else {
        return;
    };
    match advance(game, &mut run) {
        Ok(false) => STATE.with_borrow_mut(|s| s.run = Some(run)),
        Ok(true) => stop(game, run, "Block goal reached".into()),
        Err(reason) => stop(game, run, reason),
    }
}
pub(super) fn status() -> serde_json::Value {
    STATE.with_borrow(|s|serde_json::json!({"active":s.run.is_some(),"count":s.run.as_ref().map_or(s.count,|r|r.count),"goal":s.run.as_ref().map(|r|r.settings.goal),"phase":s.run.as_ref().map(|r|r.message.as_str()),"result":s.result}))
}
pub(super) fn segments() -> Vec<[[f64; 3]; 2]> {
    STATE.with_borrow(|s| {
        let Some(r) = &s.run else {
            return Vec::new();
        };
        if !r.settings.preview {
            return Vec::new();
        }
        let mut last = r.stand.map(|v| f64::from(v) + 0.5);
        let mut lines = Vec::new();
        for a in r.actions.iter().take(256) {
            let p = match a {
                Action::Walk(p) | Action::Mine(p, _) => p.map(|v| f64::from(v) + 0.5),
            };
            lines.push([last, p]);
            last = p;
        }
        lines
    })
}
