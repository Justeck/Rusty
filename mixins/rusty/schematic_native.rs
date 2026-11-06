// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Transient construction: received-state surveys, ordinary walking, inventory
// receipts and all-write-checked normal placement. Nothing re-arms itself.
use super::{
    Game,
    agent::rusty_schematic_agent as host,
    remote::{rusty_build_actions as action, rusty_inventory_actions as inventory},
    rusty_schematic::{BlockState, Schematic},
    rusty_schematic_io as io,
};
use horizon_client::{blocks::Fluid, clipboard::Clipboard, gizmos::Gizmos, items::ItemStack};
use horizon_client_render::mesher::{
    rusty, rusty_freecam,
    rusty_mining::{Cell as Terrain, Grid, Node},
    rusty_navigation::Scope,
    rusty_schematic_build as core,
};
use horizon_world::chunk::BlockState as NativeState;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{Receiver, TryRecvError},
    },
    time::{Duration, Instant},
};
const SURVEY_BUDGET: usize = 2048;
const PROBE_BUDGET: usize = 4;
const BUILD_PERMISSION_ERROR: &str = "Construction requires Survival or Creative build permission";
#[derive(Clone)]
struct Palette {
    state: u32,
    item: Option<String>,
    cost: u32,
    max_stack: u32,
    manual: bool,
    skip: bool,
}
#[derive(Clone, Copy)]
struct Planned {
    at: Node,
    state: u32,
    palette: usize,
}
struct Blueprint {
    origin: Option<Node>,
    data: core::ResolvedData<Schematic, Palette>,
    cells: Vec<Planned>,
    plan: BTreeMap<Node, u32>,
    materials: BTreeMap<String, u32>,
    stacks: BTreeMap<String, u32>,
    block_totals: BTreeMap<String, u32>,
    columns: BTreeSet<(i32, i32)>,
    manual: BTreeMap<String, u32>,
}
struct Activation {
    data: core::ResolvedData<Schematic, Palette>,
    resolving: Vec<Palette>,
    needs_registry: bool,
    index: usize,
    blueprint: Option<Blueprint>,
}
struct FileJob {
    generation: u64,
    cancel: io::Cancellation,
    receiver: Receiver<Result<io::Output, String>>,
}
struct Export {
    bounds: core::Bounds,
    index: usize,
    palette: Vec<BlockState>,
    ids: BTreeMap<u32, u32>,
    blocks: Vec<u32>,
    path: Option<PathBuf>,
    overwrite: bool,
}
#[derive(Default, Clone)]
struct Survey {
    index: usize,
    counts: core::OutcomeCounts,
    candidates: Vec<usize>,
    pass: core::SurveyPass,
    ticks: usize,
}
impl std::ops::Deref for Survey {
    type Target = core::OutcomeCounts;
    fn deref(&self) -> &Self::Target {
        &self.counts
    }
}
impl std::ops::DerefMut for Survey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.counts
    }
}
struct Swap {
    fence: u64,
    receipt: core::SwapReceipt,
    source_expected: Option<ItemStack>,
    target_expected: Option<ItemStack>,
    started: Instant,
}
enum Phase {
    Survey(Survey),
    Choose(usize),
    Aim {
        cell: usize,
        attempt: usize,
    },
    Aimed {
        cell: usize,
        writes: Vec<(Node, u32)>,
        yaw: f32,
        pitch: f32,
        tick: u64,
    },
    Placed {
        writes: Vec<(Node, u32)>,
        neighbors: Vec<core::PendingNeighbor>,
        completion_candidates: Vec<Node>,
        tick: u64,
        started: Instant,
    },
    Grid {
        cell: usize,
        grid: Grid,
        index: usize,
    },
    Route {
        cell: usize,
        route: VecDeque<Node>,
    },
    Walk {
        cell: usize,
        to: Node,
        route: VecDeque<Node>,
        receiver: Receiver<String>,
        started: Instant,
    },
}
struct Run {
    phase: Phase,
    damage: core::DamageWatch<Scope>,
    stand: Node,
    health: f32,
    border: [f64; 4],
    previous_slot: usize,
    owned_slot: Option<usize>,
    owned_stack: Option<ItemStack>,
    swap: Option<Swap>,
    visited: BTreeSet<(usize, Node)>,
    unsupported: BTreeSet<usize>,
    accepted: usize,
    inventory_confirmed: usize,
    completed: BTreeSet<Node>,
    initially_matching: Option<usize>,
    started: Instant,
}
#[derive(Default)]
struct State {
    scope: Option<Scope>,
    origin: Option<Node>,
    corners: [Option<Node>; 2],
    blueprint: Option<Blueprint>,
    activation: Option<Activation>,
    file: Option<FileJob>,
    export: Option<Export>,
    generation: core::Generation,
    run: Option<Run>,
    result: String,
    survey: Survey,
    preview_scan: Survey,
    preview: Gizmos,
    preview_work: Gizmos,
    last_tick: Option<u64>,
    accepted: usize,
    inventory_confirmed: usize,
    rotation: u8,
    damage_receipts: u64,
    mirror: bool,
    proposed_orientation: Option<(u8, bool)>,
    commit_already_started: bool,
    outcome: Option<core::OutcomeSummary>,
    terminal_unsupported: BTreeSet<usize>,
    terminal_completed: BTreeSet<Node>,
    terminal_refresh_until: Option<Instant>,
}
thread_local! {static STATE:RefCell<State>=RefCell::new(State::default());static LOOK:Cell<bool>=const{Cell::new(false)};static STATS:RefCell<core::StatsFence>=RefCell::new(core::StatsFence::default());static DIRTY:RefCell<[bool;36]>=const{RefCell::new([false;36])};static BLOCK_USES:Cell<u64>=const{Cell::new(0)};}
// Observation only: every ordinary outgoing block use on this connection,
// including manual uses, is counted independently of predicted/accepted writes.
pub(super) fn block_use_sent() {
    BLOCK_USES.set(BLOCK_USES.get().saturating_add(1));
}
pub(super) fn damage_received(game: &mut Game<'_>, hurt: i32) {
    let Some(local) = game.remote.as_ref().map(|r| r.entity_id) else {
        return;
    };
    let Ok(scope) = super::rusty_navigation_native::scope(game) else {
        return;
    };
    STATE.with_borrow_mut(|s| {
        if s.scope.as_ref() != Some(&scope) || hurt != local {
            return;
        }
        s.damage_receipts = s.damage_receipts.saturating_add(1);
        if s.run
            .as_mut()
            .is_some_and(|run| run.damage.receive(&scope, Some(local), hurt))
        {
            // handle_packet runs on the game thread. Release our walking and
            // selection owner before another frame can source, aim or place.
            stop_state(
                game,
                s,
                "Received local-player damage stopped construction".into(),
            );
        }
    });
}
pub(super) fn stats_sent() {
    STATS.with_borrow_mut(|s| {
        let _ = s.request();
    });
}
pub(super) fn stats_received() {
    STATS.with_borrow_mut(core::StatsFence::response);
}
pub(super) fn stats_reset() {
    STATS.with_borrow_mut(|s| *s = core::StatsFence::default());
    BLOCK_USES.set(0);
}
pub(super) fn stats_reliable() -> bool {
    STATS.with_borrow(core::StatsFence::reliable)
}
pub(super) fn stats_last_request() -> u64 {
    STATS.with_borrow(core::StatsFence::last_request)
}
pub(super) fn active() -> bool {
    STATE.with_borrow(|s| s.run.is_some())
}
pub(super) fn busy() -> bool {
    STATE.with_borrow(|s| {
        s.run.is_some() || s.file.is_some() || s.activation.is_some() || s.export.is_some()
    })
}
pub(super) fn manual_look() {
    LOOK.set(true);
}
fn loaded(game: &Game<'_>, at: Node) -> bool {
    at[1] >= game.world.bottom_y()
        && at[1] < game.world.top_y()
        && game.world.chunks.contains_key(&(at[0] >> 4, at[2] >> 4))
        && !action::unsettled(game, at)
}
fn within(game: &Game<'_>, at: Node) -> bool {
    let [a, b, c, d] = game.border.bounds_at(1.0);
    f64::from(at[0]) >= a
        && f64::from(at[0] + 1) <= c
        && f64::from(at[2]) >= b
        && f64::from(at[2] + 1) <= d
}
fn player_node(game: &Game<'_>) -> Result<Node, String> {
    let p = game.player.as_ref().ok_or("No player")?.position;
    Ok([p.x.floor() as i32, p.y.round() as i32, p.z.floor() as i32])
}
fn terrain(game: &Game<'_>, at: Node) -> Terrain {
    if !loaded(game, at) || !within(game, at) {
        return Terrain::unknown();
    }
    let state = game.world.block(at[0], at[1], at[2]);
    terrain_state(game, state)
}
fn terrain_state(game: &Game<'_>, state: horizon_world::registry::BlockStateId) -> Terrain {
    let facts = game.res.facts();
    let f = facts.get(state);
    if facts.is_air(state) {
        return Terrain::air();
    }
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
        .any(|s| f.name.contains(s))
    {
        return Terrain::fluid();
    }
    if f.name.ends_with(":sand")
        || f.name.ends_with("_sand")
        || f.name.ends_with(":gravel")
        || f.name.contains("anvil")
        || f.name.ends_with("concrete_powder")
    {
        return Terrain::falling();
    }
    Terrain::passive(f.full_collision)
}
fn grid(game: &Game<'_>, a: Node, b: Node) -> Result<Grid, String> {
    let min = std::array::from_fn(|i| a[i].min(b[i]) - 2);
    let size =
        std::array::from_fn(|i| (a[i] - b[i]).unsigned_abs() as usize + if i == 1 { 6 } else { 5 });
    let mut grid = Grid::new(min, size)?;
    for i in 0..grid.len() {
        let at = grid.position(i);
        grid.set(at, terrain(game, at));
    }
    Ok(grid)
}
fn standing_grid(game: &Game<'_>, run: &Run) -> Result<Grid, String> {
    let mut grid = grid(game, run.stand, run.stand)?;
    if let Phase::Placed { neighbors, .. } = &run.phase {
        // Frozen receipt wait only. These at-most-16 writes were captured from
        // known safe neighbors immediately before this exact ordinary action.
        for i in 0..grid.len() {
            let at = grid.position(i);
            if !action::unsettled(game, at) {
                continue;
            }
            for neighbor in neighbors {
                if let Some(cell) = neighbor.guard_cell(
                    run.stand,
                    at,
                    action::prediction_sequence(game, at),
                    game.world.block(at[0], at[1], at[2]).0,
                    within(game, at)
                        && at[1] >= game.world.bottom_y()
                        && at[1] < game.world.top_y()
                        && game.world.chunks.contains_key(&(at[0] >> 4, at[2] >> 4)),
                ) {
                    grid.set(at, cell);
                }
            }
        }
    }
    Ok(grid)
}
fn exact_stack(a: Option<&ItemStack>, b: Option<&ItemStack>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.count == b.count && a.same_item_same_components(b),
        _ => false,
    }
}
fn bounded(stack: &ItemStack) -> bool {
    stack.count > 0
        && stack
            .data()
            .is_none_or(|d| d.wire.patch().is_some_and(|p| p.len() <= 16 * 1024))
}
pub(super) fn inventory_mark(slot: usize) {
    if slot < 36 {
        DIRTY.with_borrow_mut(|d| d[slot] = true);
    }
}
pub(super) fn inventory_menu_mark(container: i32, slot: i32) {
    if container == 0 {
        if (36..45).contains(&slot) {
            inventory_mark((slot - 36) as usize);
        } else if (9..36).contains(&slot) {
            inventory_mark(slot as usize);
        }
    }
}
pub(super) fn inventory_received(game: &Game<'_>, full: bool) {
    let dirty = DIRTY.with_borrow_mut(|d| std::mem::replace(d, [false; 36]));
    STATE.with_borrow_mut(|s| {
        if let Some(swap) = s.run.as_mut().and_then(|r| r.swap.as_mut()) {
            for (slot, expected) in [
                (swap.receipt.source, &swap.source_expected),
                (swap.receipt.target, &swap.target_expected),
            ] {
                if full || dirty[slot] {
                    swap.receipt.observe(
                        slot,
                        exact_stack(game.inventory[slot].as_ref(), expected.as_ref()),
                    );
                }
            }
        }
    });
}
fn release(game: &mut Game<'_>, run: &Run) {
    host::cancel(game);
    if run.owned_slot == Some(game.selected) && run.previous_slot < 9 {
        game.selected = run.previous_slot;
    }
}
fn stop_state(game: &mut Game<'_>, s: &mut State, message: String) {
    if let Some(run) = s.run.take() {
        finish_run(game, s, run);
    }
    s.result = message;
}
fn outcome(
    b: &Blueprint,
    survey: &Survey,
    initially_matching: Option<usize>,
) -> core::OutcomeSummary {
    core::OutcomeSummary::observed(
        &survey.counts,
        initially_matching,
        b.data.model.ignored_block_entities,
        b.data.model.ignored_entities,
        survey.index == b.cells.len()
            && survey.pass.coherent(core::survey_revision())
            && survey.unknown == 0,
    )
}
fn finish_run(game: &mut Game<'_>, s: &mut State, run: Run) {
    release(game, &run);
    s.accepted = run.accepted;
    s.inventory_confirmed = run.inventory_confirmed;
    s.outcome = s
        .blueprint
        .as_ref()
        .map(|b| outcome(b, &s.survey, run.initially_matching));
    if let Some(outcome) = s.outcome.as_mut() {
        outcome.survey_complete = false;
    }
    s.terminal_unsupported = run.unsupported;
    s.terminal_completed = run.completed;
    s.terminal_refresh_until = Some(Instant::now() + Duration::from_secs(60));
    s.preview_scan = Survey::default();
}
fn cancel_work(s: &mut State) {
    if s.activation.is_some() {
        s.origin = s.blueprint.as_ref().and_then(|b| b.origin);
    }
    s.proposed_orientation = None;
    if let Some(job) = s.file.take() {
        if !io::cancel(&job.cancel) {
            s.result
                .push_str("; export commit already began and the chosen snapshot may finish");
            s.commit_already_started = true;
        }
    }
    s.generation.cancel();
    s.activation = None;
    s.export = None;
}
pub(super) fn reset(game: &mut Game<'_>) {
    STATE.with_borrow_mut(|s| {
        stop_state(game, s, "Construction stopped".into());
        cancel_work(s);
        let commit_already_started = s.commit_already_started;
        *s = State {
            commit_already_started,
            ..State::default()
        };
    });
    LOOK.set(false);
    DIRTY.with_borrow_mut(|d| *d = [false; 36]);
}
pub(super) fn stop(game: &mut Game<'_>) {
    STATE.with_borrow_mut(|s| {
        stop_state(
            game,
            s,
            "Stopped; use Start construction to begin again".into(),
        );
        cancel_work(s);
    });
}
fn scope(game: &Game<'_>, s: &mut State) -> Result<(), String> {
    let scope = super::rusty_navigation_native::scope(game)?;
    if s.scope.as_ref().is_some_and(|old| *old != scope) {
        return Err("World scope changed; wait for the next frame".into());
    }
    s.scope = Some(scope);
    Ok(())
}
fn ready(game: &Game<'_>) -> Result<(), String> {
    if !game.native_window()
        || !game.native_on(rusty::ID)
        || game.remote.is_none()
        || game.spectator()
    {
        Err("Open an active playable world first".into())
    } else {
        Ok(())
    }
}
pub(super) fn load(game: &mut Game<'_>, path: Option<PathBuf>) -> Result<(), String> {
    ready(game)?;
    STATE.with_borrow_mut(|s| {
        scope(game, s)?;
        stop_state(game, s, "Loading schematic; construction is stopped".into());
        cancel_work(s);
        s.proposed_orientation = Some((0, false));
        let generation = s.generation.begin();
        let (cancel, receiver) = io::begin(path, None, false, horizon_world::DATA_VERSION);
        s.file = Some(FileJob {
            generation,
            cancel,
            receiver,
        });
        Ok(())
    })
}
pub(super) fn clear(game: &mut Game<'_>) -> Result<(), String> {
    STATE.with_borrow_mut(|s| {
        stop_state(game, s, "Schematic cleared".into());
        cancel_work(s);
        s.blueprint = None;
        s.outcome = None;
        s.terminal_refresh_until = None;
        s.preview = Gizmos::default();
        s.survey = Survey::default();
        s.preview_scan = Survey::default();
        s.rotation = 0;
        s.mirror = false;
    });
    Ok(())
}
fn activate(s: &mut State, model: Arc<Schematic>) {
    s.activation = Some(Activation {
        data: core::ResolvedData {
            model,
            palette: Arc::new(Vec::new()),
        },
        resolving: Vec::new(),
        needs_registry: true,
        index: 0,
        blueprint: None,
    });
    s.result = "Resolving exact registry states; preview only".into();
}
pub(super) fn origin(game: &mut Game<'_>, position: Option<Node>) -> Result<(), String> {
    ready(game)?;
    let at = if let Some(at) = position {
        at
    } else {
        let hit = game.pick().ok_or("Aim at an outside block face")?;
        if hit.face == [0; 3] {
            return Err("Aim at an outside block face".into());
        }
        std::array::from_fn(|i| hit.pos[i] + hit.face[i])
    };
    if !loaded(game, at) || !within(game, at) {
        return Err("Origin must be settled received terrain inside the border".into());
    }
    STATE.with_borrow_mut(|s| {
        scope(game, s)?;
        stop_state(game, s, "Origin changed; construction stopped".into());
        cancel_work(s);
        s.origin = Some(at);
        if let Some(b) = &s.blueprint {
            s.activation = Some(Activation {
                data: b.data.shared(),
                resolving: Vec::new(),
                needs_registry: false,
                index: 0,
                blueprint: None,
            });
            s.result = "Applying origin to shared resolved states; preview only".into();
        }
        Ok(())
    })
}
pub(super) fn transform(game: &mut Game<'_>, mirror: bool) -> Result<(), String> {
    STATE.with_borrow_mut(|s| {
        scope(game, s)?;
        let model = s
            .blueprint
            .as_ref()
            .ok_or("Load a schematic first")?
            .data
            .model
            .clone();
        stop_state(
            game,
            s,
            "Transforming exact states; construction stopped".into(),
        );
        cancel_work(s);
        let generation = s.generation.begin();
        let (cancel, receiver) = io::transform(model, if mirror { 0 } else { 1 }, mirror);
        s.file = Some(FileJob {
            generation,
            cancel,
            receiver,
        });
        s.proposed_orientation = Some(if mirror {
            ((4 - s.rotation) % 4, !s.mirror)
        } else {
            ((s.rotation + 1) % 4, s.mirror)
        });
        Ok(())
    })
}
pub(super) fn corner(game: &Game<'_>, second: bool, position: Option<Node>) -> Result<(), String> {
    ready(game)?;
    let at = position
        .or_else(|| game.pick().map(|h| h.pos))
        .ok_or("Aim at a received block to select a corner")?;
    if !loaded(game, at) || !within(game, at) {
        return Err("Corner must be settled received terrain inside the border".into());
    }
    STATE.with_borrow_mut(|s| {
        scope(game, s)?;
        let mut corners = s.corners;
        corners[usize::from(second)] = Some(at);
        if let [Some(a), Some(b)] = corners {
            core::Bounds::new(a, b)?;
        }
        s.corners = corners;
        s.result = "States-only export corner selected".into();
        Ok(())
    })
}
pub(super) fn export(
    game: &mut Game<'_>,
    path: Option<PathBuf>,
    overwrite: bool,
) -> Result<(), String> {
    ready(game)?;
    STATE.with_borrow_mut(|s| {
        scope(game, s)?;
        let bounds = core::Bounds::new(
            s.corners[0].ok_or("Set the first export corner")?,
            s.corners[1].ok_or("Set the second export corner")?,
        )?;
        stop_state(game, s, "Reading settled block states for export".into());
        cancel_work(s);
        s.export = Some(Export {
            bounds,
            index: 0,
            palette: Vec::new(),
            ids: BTreeMap::new(),
            blocks: Vec::with_capacity(bounds.len()),
            path,
            overwrite,
        });
        Ok(())
    })
}
fn available_materials(game: &Game<'_>) -> BTreeMap<String, u32> {
    let mut out = BTreeMap::new();
    for stack in game
        .inventory
        .iter()
        .take(36)
        .flatten()
        .filter(|s| bounded(s))
    {
        *out.entry(stack.item.to_string()).or_default() += u32::from(stack.count);
    }
    out
}
fn material_text(b: &Blueprint, available: &BTreeMap<String, u32>) -> String {
    let mut text = String::from("Rusty schematic materials — block states only\n");
    for (item, count) in &b.materials {
        let max = b.stacks[item];
        let (full, remainder) = core::stack_breakdown(*count, max).unwrap_or((0, *count));
        text.push_str(&format!(
            "{item}: {count} items ({full} stacks of {max} + {remainder}); inventory {}, missing {}\n",available.get(item).copied().unwrap_or(0),count.saturating_sub(available.get(item).copied().unwrap_or(0))
        ));
    }
    if !b.manual.is_empty() {
        text.push_str("\nManual / no ordinary block item:\n");
        for (state, count) in &b.manual {
            text.push_str(&format!("{state}: {count}\n"));
        }
    }
    text.push_str("\nExact block-state totals:\n");
    for (state, count) in &b.block_totals {
        text.push_str(&format!("{state}: {count}\n"));
    }
    text.push_str(&format!(
        "\nImported data not restored: {} block entities, {} entities\n",
        b.data.model.ignored_block_entities, b.data.model.ignored_entities
    ));
    text
}
pub(super) fn copy(game: &mut Game<'_>) -> Result<(), String> {
    let available = available_materials(game);
    let text = STATE
        .with_borrow(|s| s.blueprint.as_ref().map(|b| material_text(b, &available)))
        .ok_or("Load a schematic first")?;
    game.clipboard.set(&text);
    STATE.with_borrow_mut(|s| {
        s.result = "Copied sorted materials and exact block-state totals to the OS clipboard".into()
    });
    Ok(())
}
pub(super) fn start(game: &mut Game<'_>) -> Result<(), String> {
    if !game.may_build() {
        STATE.with_borrow_mut(|s| stop_state(game, s, BUILD_PERMISSION_ERROR.into()));
        return Err(BUILD_PERMISSION_ERROR.into());
    }
    ready(game)?;
    if super::rusty_mining_native::active()
        || super::rusty_builder::busy()
        || super::rusty_assist::busy()
        || super::rusty_automations::busy()
        || super::rusty_inventory::busy()
        || !host::idle(game)
    {
        return Err("Finish the other movement, build or inventory action first".into());
    }
    STATE.with_borrow_mut(|s| {
        scope(game, s)?;
        if s.run.is_some() || s.file.is_some() || s.activation.is_some() || s.export.is_some() {
            return Err("Finish the current schematic operation first".into());
        }
        s.origin.ok_or("Set schematic origin first")?;
        let b = s
            .blueprint
            .as_ref()
            .ok_or("Load and resolve a schematic first")?;
        if b.cells.is_empty() {
            return Err("Schematic has no construction blocks".into());
        }
        let stand = player_node(game)?;
        if !grid(game, stand, stand)?.safe_stand(stand) {
            return Err("Stand on settled safe ground before construction".into());
        }
        let p = game.player.as_ref().ok_or("No player")?;
        let run = Run {
            phase: Phase::Survey(Survey::default()),
            damage: core::DamageWatch::new(
                s.scope.clone().ok_or("No construction scope")?,
                game.remote.as_ref().ok_or("No connection")?.entity_id,
            ),
            stand,
            health: p.health.health(),
            border: game.border.bounds_at(1.0),
            previous_slot: game.selected,
            owned_slot: None,
            owned_stack: None,
            swap: None,
            visited: BTreeSet::new(),
            unsupported: BTreeSet::new(),
            accepted: 0,
            inventory_confirmed: 0,
            completed: BTreeSet::new(),
            initially_matching: None,
            started: Instant::now(),
        };
        guard(game, s, &run, false)?;
        LOOK.set(false);
        s.run = Some(run);
        s.accepted = 0;
        s.inventory_confirmed = 0;
        s.damage_receipts = 0;
        s.outcome = None;
        s.terminal_refresh_until = None;
        s.result = "Construction started; surveying exact received states".into();
        Ok(())
    })
}
fn guard(game: &Game<'_>, s: &State, run: &Run, manual: bool) -> Result<(), String> {
    if !game.may_build() {
        return Err(BUILD_PERMISSION_ERROR.into());
    }
    let p = game.player.as_ref().ok_or("Player disappeared")?;
    if super::rusty_navigation_native::scope(game).ok().as_ref() != s.scope.as_ref() {
        return Err("World or dimension changed".into());
    }
    if !game.focused
        || !game.native_playing()
        || super::rusty_ui::is_open()
        || !game.native_on(rusty::ID)
        || rusty_freecam::active()
        || game.remote.is_none()
        || game.spectator()
        || game.riding.is_some()
        || p.sleeping
        || !super::rusty_waypoint_walk::available(game)
    {
        return Err("Menu, focus or player context changed".into());
    }
    if manual
        || !game.physical_keys.is_empty()
        || !game.physical_buttons.is_empty()
        || super::rusty_waypoint_walk::manual(game)
    {
        return Err("Manual input took control".into());
    }
    if run.damage.received()
        || p.health.health() < run.health
        || !p.health.health().is_finite()
        || p.health.health() < f32::from(rusty::config().build.schematic_min_health)
        || p.health.food.food < i32::from(rusty::config().build.schematic_min_food)
        || p.health.dead_or_dying()
    {
        return Err("Damage, low health or low food stopped construction".into());
    }
    if game.border.bounds_at(1.0) != run.border {
        return Err("World border changed".into());
    }
    if run.started.elapsed() > Duration::from_secs(1800) {
        return Err("Construction reached the 30 minute limit".into());
    }
    if super::rusty_mining_native::active()
        || super::rusty_builder::busy()
        || super::rusty_assist::busy()
        || super::rusty_automations::busy()
        || super::rusty_inventory::busy()
        || !host::healthy(game)
        || host::current(game).is_some_and(|n| n != host::NAME)
        || game.flight.is_some()
        || game.using.is_some()
    {
        return Err("Another action owns the controls".into());
    }
    if run.owned_slot.is_some_and(|slot| game.selected != slot) {
        return Err("Hotbar selection changed".into());
    }
    if !matches!(run.phase, Phase::Walk { .. })
        && (player_node(game)? != run.stand
            || !p.ground.on_ground
            || !standing_grid(game, run)?.safe_stand(run.stand))
    {
        return Err("Standing position or safe support changed".into());
    }
    Ok(())
}

fn resolve_step(game: &Game<'_>, s: &mut State) -> Result<(), String> {
    let Some(a) = s.activation.as_mut() else {
        return Ok(());
    };
    if a.needs_registry {
        let end = (a.resolving.len() + 256).min(a.data.model.palette.len());
        while a.resolving.len() < end {
            let index = a.resolving.len();
            let descriptor = &a.data.model.palette[index];
            let id = game
                .pack
                .assets
                .registry
                .resolve(&NativeState {
                    name: descriptor.name.clone(),
                    properties: descriptor.properties.clone().into(),
                })
                .map_err(|e| format!("Unknown state {}: {e}", descriptor.descriptor()))?;
            let full = game
                .pack
                .assets
                .registry
                .state(id)
                .ok_or("Registry state missing")?;
            let facts = game.res.facts().get(id);
            let item = facts.item.to_string();
            let cost = core::material_cost(&full.properties.clone().into());
            let air = game.res.facts().is_air(id) || full.name == "minecraft:structure_void";
            let manual = !air
                && (facts.fluid != Fluid::None
                    || full.name.contains(":potted_")
                    || !game
                        .items()
                        .get(&item)
                        .is_some_and(|i| i.block.is_some() && i.consumable.is_none()));
            a.resolving.push(Palette {
                max_stack: game.items().get(&item).map_or(1, |i| i.max_stack.max(1)),
                state: id.0,
                item: (!air && !manual).then_some(item),
                cost,
                manual,
                skip: air,
            });
            Arc::get_mut(&mut a.data.model)
                .ok_or("Unresolved model unexpectedly shared")?
                .palette[index] = BlockState {
                name: full.name.clone(),
                properties: full.properties.clone().into(),
            };
        }
        if a.resolving.len() == a.data.model.palette.len() {
            a.data.palette = Arc::new(std::mem::take(&mut a.resolving));
            a.needs_registry = false;
        }
        return Ok(());
    }
    if a.blueprint.is_none() {
        a.blueprint = Some(Blueprint {
            origin: s.origin,
            data: a.data.shared(),
            cells: Vec::new(),
            plan: BTreeMap::new(),
            materials: BTreeMap::new(),
            stacks: BTreeMap::new(),
            block_totals: BTreeMap::new(),
            columns: BTreeSet::new(),
            manual: BTreeMap::new(),
        });
    }
    let b = a.blueprint.as_mut().unwrap();
    let origin = s.origin.unwrap_or([0; 3]);
    let size = a.data.model.size;
    let end = (a.index + SURVEY_BUDGET).min(a.data.model.blocks.len());
    while a.index < end {
        let index = a.index;
        a.index += 1;
        let pi = a.data.model.blocks[index] as usize;
        let p = &a.data.palette[pi];
        let descriptor = a.data.model.palette[pi].descriptor();
        *b.block_totals.entry(descriptor.clone()).or_default() += 1;
        if p.skip {
            continue;
        }
        let local = [
            (index % size[0] as usize) as i32,
            (index / (size[0] as usize * size[2] as usize)) as i32,
            (index / size[0] as usize % size[2] as usize) as i32,
        ];
        let mut at = [0; 3];
        for i in 0..3 {
            at[i] = origin[i]
                .checked_add(local[i])
                .ok_or("Schematic position overflows world coordinates")?;
        }
        if s.origin.is_some()
            && (!within(game, at) || at[1] < game.world.bottom_y() || at[1] >= game.world.top_y())
        {
            return Err("Schematic extends outside the current world border or height".into());
        }
        b.cells.push(Planned {
            at,
            state: p.state,
            palette: pi,
        });
        b.plan.insert(at, p.state);
        b.columns.insert((at[0] >> 4, at[2] >> 4));
        if p.manual {
            *b.manual.entry(descriptor).or_default() += 1;
        }
        if let Some(item) = &p.item {
            b.stacks.insert(item.clone(), p.max_stack);
            if p.cost > 0 {
                *b.materials.entry(item.clone()).or_default() += p.cost;
            }
        }
    }
    if a.index == a.data.model.blocks.len() {
        let mut a = s.activation.take().unwrap();
        s.blueprint = a.blueprint.take();
        s.outcome = None;
        s.terminal_refresh_until = None;
        if let Some((rotation, mirror)) = s.proposed_orientation.take() {
            s.rotation = rotation;
            s.mirror = mirror;
        }
        s.preview = Gizmos::default();
        s.preview_work = Gizmos::default();
        s.survey = Survey::default();
        s.preview_scan = Survey::default();
        s.result = "Schematic ready — preview only; set origin and use Start construction".into();
    }
    Ok(())
}
fn export_step(game: &Game<'_>, s: &mut State) -> Result<(), String> {
    let Some(e) = s.export.as_mut() else {
        return Ok(());
    };
    let end = (e.index + SURVEY_BUDGET).min(e.bounds.len());
    while e.index < end {
        let at = e.bounds.position(e.index);
        if !loaded(game, at) || !within(game, at) {
            return Err(format!(
                "States-only export stopped: {at:?} is unknown, unsettled or outside the border"
            ));
        }
        let id = game.world.block(at[0], at[1], at[2]);
        let pi = if let Some(pi) = e.ids.get(&id.0) {
            *pi
        } else {
            if e.palette.len() >= 65_536 {
                return Err("Export palette exceeds 65,536 states".into());
            }
            let state = game
                .pack
                .assets
                .registry
                .state(id)
                .ok_or("Export registry state is unavailable")?;
            let pi = e.palette.len() as u32;
            e.palette.push(BlockState {
                name: state.name.clone(),
                properties: state.properties.clone().into(),
            });
            e.ids.insert(id.0, pi);
            pi
        };
        e.blocks.push(pi);
        e.index += 1;
    }
    if e.index == e.bounds.len() {
        let e = s.export.take().unwrap();
        let model = Schematic {
            name: "Rusty states-only selection".into(),
            size: e.bounds.size,
            palette: e.palette,
            blocks: e.blocks,
            ignored_block_entities: 0,
            ignored_entities: 0,
        };
        let generation = s.generation.begin();
        let (cancel, receiver) = io::begin(
            e.path,
            Some(model),
            e.overwrite,
            horizon_world::DATA_VERSION,
        );
        s.file = Some(FileJob {
            generation,
            cancel,
            receiver,
        });
        s.result = "Selection captured; choose a .schem destination (block states only)".into();
    }
    Ok(())
}
fn state_kind(game: &Game<'_>, c: Planned) -> u8 {
    if !loaded(game, c.at) {
        0
    } else {
        let actual = game.world.block(c.at[0], c.at[1], c.at[2]);
        if actual.0 == c.state {
            1
        } else if game.res.facts().is_air(actual)
            || game.res.facts().get(actual).replaceable
            || action::partial(game, c.at, c.state)
        {
            2
        } else {
            3
        }
    }
}
fn survey_step(
    game: &Game<'_>,
    b: &Blueprint,
    survey: &mut Survey,
    unsupported: Option<&BTreeSet<usize>>,
    completed: Option<&BTreeSet<Node>>,
) -> bool {
    core::survey_watch(&game.world.chunks as *const _ as usize, b.columns.clone());
    survey.ticks += 1;
    let restarted = survey.pass.begin(core::survey_revision());
    if restarted {
        survey.index = 0;
        survey.counts = core::OutcomeCounts::default();
        survey.candidates.clear();
    }
    if survey.pass.exhausted || survey.ticks > 1200 {
        return restarted;
    }
    let end = (survey.index + SURVEY_BUDGET).min(b.cells.len());
    while survey.index < end {
        let index = survey.index;
        survey.index += 1;
        let cell = b.cells[index];
        let kind = state_kind(game, cell);
        let manual = b.data.palette[cell.palette].manual;
        let inaccessible = unsupported.is_some_and(|u| u.contains(&index));
        survey.counts.observe(
            kind,
            manual,
            inaccessible,
            completed.is_some_and(|c| c.contains(&cell.at)),
        );
        if kind == 2
            && survey.candidates.len() < 64
            && !manual
            && !inaccessible
            && b.data.palette[cell.palette].cost > 0
        {
            survey.candidates.push(index);
        }
    }
    restarted
}
fn select_material(
    game: &mut Game<'_>,
    run: &mut Run,
    item: &str,
    cost: u32,
) -> Result<bool, String> {
    if let Some(swap) = run.swap.as_ref() {
        if swap.receipt.rejected() {
            return Err("Server rejected an inventory swap; both stacks were checked".into());
        }
        if swap.started.elapsed() > Duration::from_secs(5) {
            return Err("Inventory server confirmation timed out".into());
        }
        if !swap.receipt.complete() && !STATS.with_borrow(|s| s.confirmed(swap.fence)) {
            return Ok(false);
        }
        let swap = run.swap.take().unwrap();
        if !exact_stack(
            game.inventory[swap.receipt.source].as_ref(),
            swap.source_expected.as_ref(),
        ) || !exact_stack(
            game.inventory[swap.receipt.target].as_ref(),
            swap.target_expected.as_ref(),
        ) {
            return Err("Inventory changed after its swap acknowledgment".into());
        }
        run.inventory_confirmed += 1;
        game.selected = swap.receipt.target;
        run.owned_slot = Some(game.selected);
        run.owned_stack = game.held().cloned();
        return Ok(true);
    }
    let enough =
        |s: &ItemStack| bounded(s) && s.item.as_ref() == item && u32::from(s.count) >= cost.min(1);
    if let Some(slot) = (0..9).find(|i| game.inventory[*i].as_ref().is_some_and(enough)) {
        game.selected = slot;
        run.owned_slot = Some(slot);
        run.owned_stack = game.held().cloned();
        return Ok(true);
    }
    let source = (9..36)
        .find(|i| game.inventory[*i].as_ref().is_some_and(enough))
        .ok_or_else(|| format!("Missing material {item}; no automatic retry"))?;
    let target = (0..9)
        .find(|i| game.inventory[*i].is_none())
        .unwrap_or(run.previous_slot.min(8));
    if game.inventory[target].as_ref().is_some_and(|s| !bounded(s)) {
        return Err("The displaced hotbar item's component data is unavailable".into());
    }
    let mut swap = Swap {
        fence: 0,
        receipt: core::SwapReceipt::new(source, target)?,
        source_expected: game.inventory[target].clone(),
        target_expected: game.inventory[source].clone(),
        started: Instant::now(),
    };
    DIRTY.with_borrow_mut(|d| {
        d[source] = false;
        d[target] = false;
    });
    swap.fence = inventory::construction_swap(game, source, target)
        .ok_or("Ordinary main-inventory swap or ordered confirmation was refused")?;
    run.swap = Some(swap);
    Ok(false)
}
fn next_survey(run: &mut Run) {
    run.phase = Phase::Survey(Survey::default());
}
fn advance(game: &mut Game<'_>, s: &mut State, run: &mut Run) -> Result<Option<String>, String> {
    guard(game, s, run, LOOK.replace(false))?;
    run.health = game.player.as_ref().unwrap().health.health();
    let b = s.blueprint.as_ref().ok_or("Schematic cleared")?;
    let phase = std::mem::replace(&mut run.phase, Phase::Survey(Survey::default()));
    match phase {
        Phase::Survey(mut survey) => {
            survey_step(
                game,
                b,
                &mut survey,
                Some(&run.unsupported),
                Some(&run.completed),
            );
            if survey.pass.exhausted || survey.ticks > 1200 {
                s.survey = survey;
                return Err(
                    "Received survey did not stabilize within its bound; unverified".into(),
                );
            }
            if survey.index < b.cells.len() {
                run.phase = Phase::Survey(survey);
                return Ok(None);
            }
            s.survey = survey.clone();
            run.initially_matching.get_or_insert(survey.matching);
            s.outcome = Some(outcome(b, &survey, run.initially_matching));
            if survey.unknown > 0 {
                return Err(format!(
                    "{} schematic cells are unknown or unsettled; construction stopped",
                    survey.unknown
                ));
            }
            if survey.matching == b.cells.len() {
                return Ok(Some(format!(
                    "Exact block-state verification complete: {} matching cells; {} acknowledged writes. Imported block-entity/entity data still requires manual restoration ({}/{})",
                    survey.matching,
                    run.accepted,
                    b.data.model.ignored_block_entities,
                    b.data.model.ignored_entities
                )));
            }
            let p = run.stand;
            survey.candidates.sort_by_key(|i| {
                let c = b.cells[*i];
                (
                    c.at[1],
                    (i64::from(c.at[0]) - i64::from(p[0])).pow(2)
                        + (i64::from(c.at[2]) - i64::from(p[2])).pow(2),
                )
            });
            if let Some(index) = survey.candidates.first() {
                run.phase = Phase::Choose(*index);
            } else {
                return Ok(Some(format!(
                    "Construction stopped incomplete: {} initially matching, {} now matching, {} completed, {} occupied, {} missing ({} inaccessible, {} manual), {} unknown. No demolition or scaffolding was attempted",
                    run.initially_matching.unwrap_or(0),
                    survey.matching,
                    survey.completed,
                    survey.occupied,
                    survey.missing,
                    survey.unsupported,
                    survey.manual,
                    survey.unknown
                )));
            }
        }
        Phase::Choose(cell) => {
            let c = b.cells[cell];
            if state_kind(game, c) == 1 {
                next_survey(run);
                return Ok(None);
            }
            if !within(game, c.at) {
                return Err("Planned cell moved outside the world border".into());
            }
            let palette = &b.data.palette[c.palette];
            let item = palette
                .item
                .as_deref()
                .ok_or("This state needs manual construction")?;
            if select_material(game, run, item, palette.cost)? {
                run.phase = Phase::Aim { cell, attempt: 0 };
            } else {
                run.phase = Phase::Choose(cell);
            }
        }
        Phase::Aim { cell, mut attempt } => {
            if !exact_stack(game.held(), run.owned_stack.as_ref()) {
                return Err("Selected material or components changed before placement".into());
            }
            if game.use_delay > 0 {
                run.phase = Phase::Aim { cell, attempt };
                return Ok(None);
            }
            let target = b.cells[cell].at;
            if !loaded(game, target) {
                return Err("Planned target became unknown or unsettled".into());
            }
            if state_kind(game, b.cells[cell]) == 3 {
                return Err("Planned target became occupied before placement".into());
            }
            let mut aimed = None;
            for _ in 0..PROBE_BUDGET {
                if attempt >= 60 {
                    break;
                }
                let writes = host::aim_candidate(game, target, &b.plan, attempt);
                attempt += 1;
                if let Some(writes) = writes {
                    aimed = Some(writes);
                    break;
                }
            }
            if let Some(writes) = aimed {
                run.phase = Phase::Aimed {
                    cell,
                    writes,
                    yaw: game.yaw,
                    pitch: game.pitch,
                    tick: game.ticks,
                };
            } else if attempt < 60 {
                run.phase = Phase::Aim { cell, attempt };
            } else {
                run.visited.insert((cell, run.stand));
                if run
                    .visited
                    .range((cell, [i32::MIN; 3])..=(cell, [i32::MAX; 3]))
                    .count()
                    >= 16
                {
                    run.unsupported.insert(cell);
                    next_survey(run);
                } else {
                    let origin = [
                        run.stand[0] - 12,
                        (run.stand[1] - 6).max(game.world.bottom_y()),
                        run.stand[2] - 12,
                    ];
                    let height = (game.world.top_y() - origin[1]).min(16) as usize;
                    run.phase = Phase::Grid {
                        cell,
                        grid: Grid::new(origin, [25, height, 25])?,
                        index: 0,
                    };
                }
            }
        }
        Phase::Aimed {
            cell,
            writes,
            yaw,
            pitch,
            tick,
        } => {
            if game.yaw != yaw
                || game.pitch != pitch
                || !exact_stack(game.held(), run.owned_stack.as_ref())
            {
                return Err("Aim or held item changed before dispatch".into());
            }
            if game.ticks <= tick || game.use_delay > 0 {
                run.phase = Phase::Aimed {
                    cell,
                    writes,
                    yaw,
                    pitch,
                    tick,
                };
            } else {
                let mut context = Vec::new();
                let mut completion_candidates = Vec::new();
                for &(at, state) in &writes {
                    let dx = (i64::from(at[0]) - i64::from(run.stand[0])).abs();
                    let dz = (i64::from(at[2]) - i64::from(run.stand[2])).abs();
                    let dy = i64::from(at[1]) - i64::from(run.stand[1]);
                    if dx + dz == 0 && (-1..=1).contains(&dy) {
                        return Err("Placement would change current body or support cells".into());
                    }
                    if dx + dz == 1 && (0..=1).contains(&dy) {
                        let before = terrain(game, at);
                        let after =
                            terrain_state(game, horizon_world::registry::BlockStateId(state));
                        if !before.safe_neighbor() || !after.safe_neighbor() {
                            return Err(
                                "Placement would make a current standing neighbor unsafe".into()
                            );
                        }
                        context.push((at, state, before, after));
                    }
                    if b.plan.get(&at) == Some(&state)
                        && game.world.block(at[0], at[1], at[2]).0 != state
                    {
                        completion_candidates.push(at);
                    }
                }
                action::place_exact(game, b.cells[cell].at, &b.plan, &writes)?;
                let mut neighbors = Vec::new();
                for (at, state, before, after) in context {
                    let sequence = action::prediction_sequence(game, at)
                        .ok_or("Owned placement prediction disappeared")?;
                    neighbors.push(
                        core::PendingNeighbor::capture(
                            run.stand, at, sequence, state, before, after,
                        )
                        .ok_or("Placement neighbor context changed")?,
                    );
                }
                run.phase = Phase::Placed {
                    writes,
                    neighbors,
                    completion_candidates,
                    tick: game.ticks,
                    started: Instant::now(),
                };
            }
        }
        Phase::Placed {
            writes,
            neighbors,
            completion_candidates,
            tick,
            started,
        } => {
            if started.elapsed() > Duration::from_secs(5) {
                return Err("Placement acknowledgment timed out; no automatic retry".into());
            }
            if game.ticks > tick && writes.iter().all(|(at, _)| loaded(game, *at)) {
                if writes
                    .iter()
                    .any(|(at, state)| game.world.block(at[0], at[1], at[2]).0 != *state)
                {
                    return Err(
                        "Server settled a placement without every expected state; stopped".into(),
                    );
                }
                run.accepted += writes.len();
                run.completed.extend(completion_candidates);
                run.owned_stack = game.held().cloned();
                next_survey(run);
            } else {
                run.phase = Phase::Placed {
                    writes,
                    neighbors,
                    completion_candidates,
                    tick,
                    started,
                };
            }
        }
        Phase::Grid {
            cell,
            mut grid,
            mut index,
        } => {
            let end = (index + SURVEY_BUDGET).min(grid.len());
            while index < end {
                let at = grid.position(index);
                grid.set(at, terrain(game, at));
                index += 1;
            }
            if index < grid.len() {
                run.phase = Phase::Grid { cell, grid, index };
            } else {
                let target = b.cells[cell].at;
                let descriptor = &b.data.model.palette[b.cells[cell].palette];
                let route = core::walk_route(
                    &grid,
                    run.stand,
                    |at| {
                        at != run.stand
                            && core::approach_matches(
                                &descriptor.name,
                                &descriptor.properties,
                                at,
                                target,
                            )
                            && !run.visited.contains(&(cell, at))
                            && (at[0] - target[0]).abs() <= 3
                            && (at[2] - target[2]).abs() <= 3
                            && (at[1] - target[1]).abs() <= 3
                    },
                    4096,
                );
                if let Some(route) = route {
                    run.phase = Phase::Route { cell, route };
                } else {
                    run.unsupported.insert(cell);
                    next_survey(run);
                }
            }
        }
        Phase::Route { cell, mut route } => {
            if let Some(to) = route.pop_front() {
                if !grid(game, run.stand, to)?.can_walk(run.stand, to) {
                    return Err("Received walking edge became unsafe".into());
                }
                let receiver = host::walk(game, run.stand, to)?;
                run.phase = Phase::Walk {
                    cell,
                    to,
                    route,
                    receiver,
                    started: Instant::now(),
                };
            } else {
                run.phase = Phase::Choose(cell);
            }
        }
        Phase::Walk {
            cell,
            to,
            route,
            receiver,
            started,
        } => {
            if !grid(game, run.stand, to)?.can_walk(run.stand, to) {
                return Err("Walking support, hazard or unknown terrain changed".into());
            }
            if started.elapsed() > Duration::from_secs(5) {
                return Err("Ordinary walking timed out".into());
            }
            match receiver.try_recv() {
                Ok(result) => {
                    let value: serde_json::Value =
                        serde_json::from_str(&result).unwrap_or_default();
                    if value.get("error").is_some() {
                        return Err(format!("Walking stopped: {result}"));
                    }
                    let p = game.player.as_ref().unwrap();
                    if !p.ground.on_ground || player_node(game)? != to {
                        return Err("Walking ended without a settled grounded arrival".into());
                    }
                    run.stand = to;
                    run.phase = Phase::Route { cell, route };
                }
                Err(TryRecvError::Empty) => {
                    run.phase = Phase::Walk {
                        cell,
                        to,
                        route,
                        receiver,
                        started,
                    };
                }
                Err(TryRecvError::Disconnected) => {
                    return Err("Walking owner ended without acknowledgment".into());
                }
            }
        }
    }
    Ok(None)
}
fn cube(out: &mut Gizmos, at: Node, color: u32) {
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
fn file_result(s: &mut State) {
    let Some(job) = s.file.as_ref() else {
        return;
    };
    let result = match job.receiver.try_recv() {
        Ok(result) => result,
        Err(TryRecvError::Empty) => return,
        Err(TryRecvError::Disconnected) => Err("File worker ended without a result".into()),
    };
    let generation = job.generation;
    s.file = None;
    if !s.generation.accepts(generation) {
        return;
    }
    s.generation.cancel();
    match result {
        Ok(io::Output::Loaded(name, model)) => {
            s.result = format!("Read {name}; resolving registry states");
            activate(s, Arc::new(model));
        }
        Ok(io::Output::Saved(name)) => {
            s.result = format!("Saved {name} — block states only; no entities/container contents")
        }
        Ok(io::Output::Cancelled) => {
            s.proposed_orientation = None;
            s.result = "File chooser cancelled; construction remains stopped".into();
        }
        Err(error) => {
            s.proposed_orientation = None;
            s.result = error;
        }
    }
}
pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    let current = super::rusty_navigation_native::scope(game).ok();
    STATE.with_borrow_mut(|s| {
        if !game.native_on(rusty::ID)
            || game.native_player().is_none()
            || s.scope
                .as_ref()
                .is_some_and(|old| Some(old) != current.as_ref())
        {
            stop_state(
                game,
                s,
                "World changed; schematic and export selection cleared".into(),
            );
            cancel_work(s);
            let commit_already_started = s.commit_already_started;
            *s = State {
                commit_already_started,
                ..State::default()
            };
            return;
        }
        // Immediate safety gates run on every hook; budgeted work runs once per tick.
        if let Some(run) = s.run.as_ref() {
            if let Err(error) = guard(game, s, run, LOOK.get()) {
                stop_state(game, s, error);
            }
        }
        if s.last_tick == Some(game.ticks) {
            return;
        }
        s.last_tick = Some(game.ticks);
        file_result(s);
        if let Err(error) = resolve_step(game, s) {
            cancel_work(s);
            s.result = error;
        }
        if let Err(error) = export_step(game, s) {
            s.export = None;
            s.result = error;
        }
        if let Some(mut run) = s.run.take() {
            match advance(game, s, &mut run) {
                Ok(None) => {
                    s.result = format!(
                        "Construction: {} acknowledged writes; {} inaccessible candidates",
                        run.accepted,
                        run.unsupported.len()
                    );
                    s.run = Some(run);
                }
                Ok(Some(result)) | Err(result) => {
                    finish_run(game, s, run);
                    s.result = result;
                }
            }
        }
        if s.run.is_none() && s.activation.is_none() && s.origin.is_some() {
            if let Some(b) = &s.blueprint {
                let mut start = s.preview_scan.index;
                if start == 0 {
                    s.preview_work = Gizmos::default();
                }
                let restarted = survey_step(
                    game,
                    b,
                    &mut s.preview_scan,
                    Some(&s.terminal_unsupported),
                    Some(&s.terminal_completed),
                );
                if restarted {
                    start = 0;
                    s.preview_work = Gizmos::default();
                }
                if s.preview_scan.pass.exhausted
                    || s.preview_scan.ticks > 1200
                    || s.terminal_refresh_until
                        .is_some_and(|d| Instant::now() >= d)
                {
                    if let Some(o) = s.outcome.as_mut() {
                        o.survey_complete = false;
                    }
                    s.terminal_refresh_until = None;
                    s.preview_scan = Survey::default();
                    start = 0;
                }
                let p = game.native_player().unwrap();
                for c in &b.cells[start..s.preview_scan.index] {
                    if s.preview_work.lines.len() >= 128 * 24 {
                        break;
                    }
                    if (f64::from(c.at[0]) - p.x).abs() > 32.
                        || (f64::from(c.at[2]) - p.z).abs() > 32.
                    {
                        continue;
                    }
                    let color = match state_kind(game, *c) {
                        0 => 0xffe0ab69,
                        1 => 0xff76be91,
                        2 => 0xff7cbcff,
                        _ => 0xffec7777,
                    };
                    cube(&mut s.preview_work, c.at, color);
                }
                if s.preview_scan.index == b.cells.len() {
                    s.preview = std::mem::take(&mut s.preview_work);
                    s.survey = std::mem::take(&mut s.preview_scan);
                    if let Some(deadline) = s.terminal_refresh_until {
                        let initially = s.outcome.as_ref().and_then(|o| o.initially_matching);
                        s.outcome = Some(outcome(b, &s.survey, initially));
                        if s.survey.unknown == 0 || Instant::now() >= deadline {
                            s.terminal_refresh_until = None;
                        }
                    }
                }
            }
        }
    });
}
pub(super) fn append_gizmos(game: &Game<'_>, out: &mut Gizmos) {
    if game.native_window() && game.native_on(rusty::ID) && rusty::config().build.enabled {
        STATE.with_borrow(|s| out.lines.extend_from_slice(&s.preview.lines));
    }
}
pub(super) fn materials_details(game: &Game<'_>) -> String {
    let available = available_materials(game);
    STATE.with_borrow(|s| {
        let Some(b) = &s.blueprint else { return "Load a schematic first".into(); };
        let mut text = format!("Full-plan stock comparison ({} item types)\nBuilt world blocks are not deducted.\nEscape, then Copy materials for all IDs/states/stacks.\n", b.materials.len());
        for (item, count) in &b.materials {
            let stock = available.get(item).copied().unwrap_or(0);
            let name = item.strip_prefix("minecraft:").unwrap_or(item).replace('_', " ");
            text.push_str(&format!("{name}: need {count} / have {stock} / short {}\n", count.saturating_sub(stock)));
        }
        text.push_str(&format!("Manual state types: {}; data details: {} block entities / {} entities.\nFull list: Escape, then Copy materials.",
            b.manual.len(), b.data.model.ignored_block_entities, b.data.model.ignored_entities));
        text
    })
}
pub(super) fn details() -> String {
    STATE.with_borrow(|s| {
        let mut text = format!(
            "{} | {} | origin {:?} | export {:?} / {:?}",
            s.blueprint
                .as_ref()
                .map_or("No schematic", |b| b.data.model.name.as_str()),
            s.result,
            s.origin,
            s.corners[0],
            s.corners[1]
        );
        if let Some(o) = &s.outcome {
            text.push_str(&format!("\nRun outcome: initially matching {:?}; finally matching {}; completed {}; occupied {}; missing {} (inaccessible {}, manual {}); unknown {}. Imported data still manual: {} block entities / {} entities. Settled survey: {}.", o.initially_matching, o.finally_matching, o.completed, o.occupied, o.remaining_missing, o.unsupported_inaccessible, o.remaining_manual, o.unknown, o.ignored_block_entities, o.ignored_entities, o.survey_complete));
        }
        if s.commit_already_started { text.push_str("\nAn authorized export commit had already begun when cancellation arrived; that chosen snapshot may finish saving."); }
        text
    })
}
pub(super) fn command(
    game: &mut Game<'_>,
    name: &str,
    args: &serde_json::Value,
) -> Result<(), String> {
    let position = if let Some(value) = args.get("position") {
        Some(
            serde_json::from_value::<Node>(value.clone())
                .map_err(|_| "position must contain three integer coordinates")?,
        )
    } else {
        None
    };
    let path = args.get("path").and_then(|v| v.as_str()).map(PathBuf::from);
    match name.trim_start_matches("schematic_") {
        "load" => load(game, path),
        "clear" => clear(game),
        "origin" => origin(game, position),
        "rotate" => transform(game, false),
        "mirror" => transform(game, true),
        "copy" => copy(game),
        "start" => start(game),
        "stop" => {
            stop(game);
            Ok(())
        }
        "first" => corner(game, false, position),
        "second" => corner(game, true, position),
        "save" => export(
            game,
            path,
            args.get("overwrite")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        ),
        "status" => Ok(()),
        _ => Err("Unknown schematic operation".into()),
    }
}
pub(super) fn snapshot() -> serde_json::Value {
    STATE.with_borrow(|s| {
        let (sent,received,reliable)=STATS.with_borrow(core::StatsFence::snapshot);
        let pending_swap=s.run.as_ref().and_then(|r|r.swap.as_ref()).map(|swap|serde_json::json!({
            "source":swap.receipt.source,"hotbar":swap.receipt.target,"fence_ordinal":swap.fence,
            "both_slots_received":swap.receipt.complete(),"contradictory_receipt":swap.receipt.rejected(),
            "source_expected":swap.source_expected.as_ref().map(|s|serde_json::json!({"item":s.item.as_ref(),"count":s.count})),
            "target_expected":swap.target_expected.as_ref().map(|s|serde_json::json!({"item":s.item.as_ref(),"count":s.count})),
        }));
        let phase=s.run.as_ref().map(|r|match r.phase{
            Phase::Survey(_)=>"survey",Phase::Choose(_)=>if r.swap.is_some(){"inventory_ack"}else{"material"},
            Phase::Aim{..}=>"aim",Phase::Aimed{..}=>"aimed",Phase::Placed{..}=>"placement_ack",
            Phase::Grid{..}=>"route_survey",Phase::Route{..}=>"route",Phase::Walk{..}=>"walking",
        });
        serde_json::json!({
            "active":s.run.is_some(),"phase":phase,"survey_scanned":s.run.as_ref().and_then(|r|if let Phase::Survey(v)=&r.phase{Some(v.index)}else{None}),"stats_fence":{"sent":sent,"received":received,"reliable":reliable},"pending_swap":pending_swap,"scope":s.scope.as_ref().map(|v|&v.dimension),
            "origin":s.origin,"corners":s.corners,"name":s.blueprint.as_ref().map(|b|&b.data.model.name),
            "size":s.blueprint.as_ref().map(|b|b.data.model.size),"cells":s.blueprint.as_ref().map_or(0,|b|b.cells.len()),
            "materials":s.blueprint.as_ref().map(|b|&b.materials),"manual":s.blueprint.as_ref().map(|b|&b.manual),
            "rotation":s.rotation,"mirror":s.mirror,
            "outcome":s.outcome,"terminal_survey_pending":s.terminal_refresh_until.is_some(),
            "minimum_health":rusty::config().build.schematic_min_health,"minimum_food":rusty::config().build.schematic_min_food,
            "matching":s.survey.matching,"missing":s.survey.missing,"occupied":s.survey.occupied,"unknown":s.survey.unknown,
            "accepted_writes":s.run.as_ref().map_or(s.accepted,|r|r.accepted),
            "damage_receipts":s.damage_receipts,"ordinary_block_uses":BLOCK_USES.get(),
            "inventory_confirmed":s.run.as_ref().map_or(s.inventory_confirmed,|r|r.inventory_confirmed),
            "inventory_confirmation":"actual slots or ordered vanilla statistics response plus exact both-slot validation",
            "file_pending":s.file.is_some(),"export_commit_already_started":s.commit_already_started,"resolving":s.activation.is_some(),"export_scanned":s.export.as_ref().map(|e|e.index),
            "result":s.result,"survey_budget":SURVEY_BUDGET,"probe_budget":PROBE_BUDGET,
        })
    })
}
