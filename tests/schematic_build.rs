// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
pub use rusty_horizon::rusty_mining;
#[path = "../mixins/rusty/schematic_build.rs"]
mod build;
use build::*;
use std::collections::BTreeMap;

#[test]
fn received_damage_latches_despite_absorption_or_healing_and_is_scope_local() {
    let scope = ("server-a", "overworld");
    let mut run = DamageWatch::new(scope, 17);
    // Other entities, dimension/server changes, and a replacement local player
    // must not turn a foreign receipt into authority over this captured run.
    assert!(!run.receive(&scope, Some(17), 18));
    assert!(!run.receive(&("server-a", "nether"), Some(17), 17));
    assert!(!run.receive(&("server-b", "overworld"), Some(17), 17));
    assert!(!run.receive(&scope, Some(19), 17));
    assert!(!run.receive(&scope, None, 17));
    assert!(!run.received());
    // A real local damage receipt is decisive even when the resulting health
    // is unchanged (absorption), then healed before the next frame.
    assert!(run.receive(&scope, Some(17), 17));
    assert!(run.received());
    assert!(!run.receive(&("server-a", "nether"), Some(17), 17));
    assert!(
        run.received(),
        "later foreign traffic cannot clear a damage stop"
    );
    // Explicit restart captures a fresh run; a past hit cannot auto-arm or
    // cancel that run. The next genuine local receipt must still stop it.
    let mut restarted = DamageWatch::new(scope, 17);
    assert!(!restarted.received());
    assert!(restarted.receive(&scope, Some(17), 17));
    assert!(restarted.received());
}

#[test]
fn repeated_coupled_dry_writes_compare_identity_not_hashmap_iteration_order() {
    let planned = [([2, 200, 9], 101), ([2, 200, 10], 102)];
    let reversed = [planned[1], planned[0]];
    assert!(
        same_writes(&planned, &reversed),
        "unchanged bed writes must survive reversed HashMap iteration order"
    );
    assert!(same_writes(&planned, &planned));
    assert!(!same_writes(&planned, &[planned[0], ([2, 200, 10], 103)]));
    assert!(!same_writes(&planned, &[planned[0], ([2, 200, 11], 102)]));
    assert!(!same_writes(&planned, &planned[..1]));
    assert!(!same_writes(
        &planned,
        &[planned[0], planned[1], ([8, 200, 8], 101)]
    ));
    assert!(!same_writes(&planned, &[planned[0], planned[0]]));
    assert!(!same_writes(
        &[planned[0], planned[0]],
        &[planned[0], planned[0]]
    ));
    assert!(!same_writes(
        &[planned[0], ([2, 200, 9], 102)],
        &[planned[0], ([2, 200, 9], 102)]
    ));
    assert!(!same_writes(&[], &[]));
    let oversized: Vec<_> = (0..17).map(|x| ([x, 200, 0], 101)).collect();
    assert!(!same_writes(&oversized, &oversized));
}

#[test]
fn schematic_safety_settings_validate_restore_and_preserve_other_controls() {
    use rusty_horizon::core::Config;
    let mut c = Config::default();
    let v = serde_json::to_value(&c).unwrap();
    assert_eq!(v["build"]["schematic_min_health"], 12);
    assert_eq!(v["build"]["schematic_min_food"], 10);
    c = Config::from_json(
        r#"{"build":{"schematic_min_health":18,"schematic_min_food":17,"width":9}}"#,
    )
    .unwrap();
    c.restore_default_path("/build/schematic_min_health")
        .unwrap();
    c.restore_default_path("/build/schematic_min_food").unwrap();
    let v = serde_json::to_value(&c).unwrap();
    assert_eq!(v["build"]["schematic_min_health"], 12);
    assert_eq!(v["build"]["schematic_min_food"], 10);
    assert_eq!(v["build"]["width"], 9);
    assert!(Config::from_json(r#"{"build":{"schematic_min_health":0}}"#).is_err());
    assert!(Config::from_json(r#"{"build":{"schematic_min_food":21}}"#).is_err());
}

#[test]
fn origin_activation_cannot_trigger_a_whole_schematic_copy() {
    let native = include_str!("../mixins/rusty/schematic_native.rs");
    assert!(
        !native.contains("Arc::make_mut"),
        "origin activation can deep-clone a million-cell model on the game thread"
    );
    assert!(
        !native.contains("palette: a.palette.clone()"),
        "activation deep-clones all resolved palette entries"
    );
    let origin = native
        .split("pub(super) fn origin(")
        .nth(1)
        .unwrap()
        .split("pub(super) fn transform(")
        .next()
        .unwrap();
    assert!(origin.contains("data: b.data.shared()"));
    assert!(
        origin.contains("needs_registry: false"),
        "origin must bypass registry mutation"
    );
}

#[test]
fn million_cell_origin_reactivation_shares_noncloneable_resolved_data() {
    use std::sync::Arc;
    // These deliberately do not implement Clone: sharing must never require it.
    struct Model(Vec<u32>);
    struct Palette(String);
    let data = ResolvedData {
        model: Arc::new(Model(vec![0; 1_048_576])),
        palette: Arc::new((0..65_536).map(|i| Palette(i.to_string())).collect()),
    };
    let pending = data.shared();
    assert!(Arc::ptr_eq(&data.model, &pending.model));
    assert!(Arc::ptr_eq(&data.palette, &pending.palette));
    assert_eq!(pending.model.0.len(), 1_048_576);
    assert_eq!(pending.palette[65_535].0, "65535");
    drop(pending); // Cancellation leaves the accepted data intact.
    assert_eq!(Arc::strong_count(&data.model), 1);
}

#[test]
fn only_exact_owned_pending_side_cells_can_survive_the_standing_guard() {
    use rusty_mining::{Cell, Grid};
    let stand = [1, 200, 7];
    let at = [1, 200, 6];
    let mut grid = Grid::new([-1, 198, 5], [5, 6, 5]).unwrap();
    for i in 0..grid.len() {
        let p = grid.position(i);
        grid.set(
            p,
            if p[1] < 200 {
                Cell::wall()
            } else {
                Cell::air()
            },
        );
    }
    grid.set(at, Cell::unknown());
    assert!(
        !grid.safe_stand(stand),
        "reproduces the first actual stair stop"
    );
    let owned =
        PendingNeighbor::capture(stand, at, 17, 91, Cell::air(), Cell::passive(false)).unwrap();
    let overlay = owned.guard_cell(stand, at, Some(17), 91, true).unwrap();
    grid.set(at, overlay);
    assert!(grid.safe_stand(stand));
    assert!(owned.guard_cell(stand, at, Some(18), 91, true).is_none());
    assert!(owned.guard_cell(stand, at, Some(17), 92, true).is_none());
    assert!(owned.guard_cell(stand, at, Some(17), 91, false).is_none());
    assert!(
        owned
            .guard_cell([2, 200, 7], at, Some(17), 91, true)
            .is_none()
    );
    for bad in [Cell::unknown(), Cell::fluid(), Cell::falling()] {
        assert!(PendingNeighbor::capture(stand, at, 17, 91, bad, Cell::air()).is_none());
        assert!(PendingNeighbor::capture(stand, at, 17, 91, Cell::air(), bad).is_none());
    }
    for body in [stand, [1, 201, 7], [1, 199, 7]] {
        assert!(PendingNeighbor::capture(stand, body, 17, 91, Cell::air(), Cell::air()).is_none());
    }
    grid.set([2, 200, 7], Cell::unknown());
    assert!(!grid.safe_stand(stand), "foreign unknown remains unsafe");
}

#[test]
fn terminal_summary_retains_distinct_outcomes_after_run_is_gone() {
    let mut counts = OutcomeCounts::default();
    // Three existing correct cells, two received completions, an occupied cell,
    // and six missing (two inaccessible, one manual). Unknown never completes.
    for _ in 0..3 {
        counts.observe(1, false, false, false);
    }
    for _ in 0..2 {
        counts.observe(1, false, false, true);
    }
    counts.observe(3, true, true, false);
    for i in 0..6 {
        counts.observe(2, i == 0, i == 1 || i == 2, true);
    }
    let summary = OutcomeSummary::observed(&counts, Some(3), 4, 2, true);
    let saved = serde_json::to_value(&summary).unwrap();
    assert_eq!(saved["completed"], 2);
    assert_eq!(saved["unsupported_inaccessible"], 2);
    assert_eq!(saved["remaining_manual"], 1);
    assert_eq!(saved["initially_matching"], 3);
    assert_eq!(saved["ignored_block_entities"], 4);
    assert_eq!(saved["occupied"], 1);
    assert_eq!(saved["remaining_missing"], 6);
    counts.observe(0, false, false, true);
    let unfinished = OutcomeSummary::observed(&counts, Some(3), 4, 2, true);
    assert!(!unfinished.survey_complete);
    assert_eq!(
        unfinished.completed, 2,
        "an unknown or missing cell cannot count as a completion"
    );
}

#[test]
fn every_dry_write_must_match_the_plan_before_dispatch() {
    let plan = BTreeMap::from([([0, 1, 0], 5), ([0, 2, 0], 6)]);
    assert!(validate_writes(&plan, [0, 1, 0], &[([0, 1, 0], 5), ([0, 2, 0], 6)]).is_ok());
    assert!(validate_writes(&plan, [0, 1, 0], &[([0, 1, 0], 5), ([0, 2, 0], 7)]).is_err());
    assert!(validate_writes(&plan, [0, 1, 0], &[([0, 1, 0], 5), ([1, 1, 0], 6)]).is_err());
    assert!(validate_writes(&plan, [0, 1, 0], &[]).is_err());
    assert!(validate_writes(&plan, [0, 1, 0], &[([0, 1, 0], 5), ([0, 1, 0], 5)]).is_err());
}
#[test]
fn source_and_destination_need_separate_actual_receipts() {
    let mut receipt = SwapReceipt::new(12, 2).unwrap();
    assert!(!receipt.observe(2, true));
    assert!(!receipt.complete());
    assert!(!receipt.observe(10, true));
    assert!(receipt.observe(12, true));
    assert!(receipt.complete());
    receipt.observe(2, false);
    assert!(receipt.rejected());
    assert!(!receipt.complete());
    assert!(SwapReceipt::new(1, 2).is_err());
}
#[test]
fn materials_use_host_mapping_and_coupled_double_costs() {
    let mut totals = BTreeMap::new();
    add_material(
        &mut totals,
        "minecraft:smooth_stone_slab",
        &BTreeMap::from([("type".into(), "double".into())]),
        3,
    );
    add_material(
        &mut totals,
        "minecraft:oak_door",
        &BTreeMap::from([("half".into(), "upper".into())]),
        2,
    );
    add_material(
        &mut totals,
        "minecraft:oak_door",
        &BTreeMap::from([("half".into(), "lower".into())]),
        2,
    );
    add_material(
        &mut totals,
        "minecraft:red_bed",
        &BTreeMap::from([("part".into(), "head".into())]),
        1,
    );
    add_material(
        &mut totals,
        "minecraft:red_bed",
        &BTreeMap::from([("part".into(), "foot".into())]),
        1,
    );
    assert_eq!(
        totals,
        BTreeMap::from([
            ("minecraft:oak_door".into(), 2),
            ("minecraft:red_bed".into(), 1),
            ("minecraft:smooth_stone_slab".into(), 6)
        ])
    );
    assert_eq!(
        stack_breakdown(3, 1),
        Some((3, 0)),
        "unstackable beds use their registry capacity"
    );
    assert_eq!(stack_breakdown(130, 64), Some((2, 2)));
    assert_eq!(stack_breakdown(1, 0), None);
}
#[test]
fn inclusive_export_selection_has_checked_bounds() {
    let b = Bounds::new([-2, 4, 9], [1, 4, 10]).unwrap();
    assert_eq!(b.size, [4, 1, 2]);
    assert_eq!(b.position(7), [1, 4, 10]);
    assert!(Bounds::new([0, 0, 0], [256, 0, 0]).is_err());
    assert!(Bounds::new([i32::MIN; 3], [i32::MAX; 3]).is_err());
    assert!(Bounds::new([0; 3], [255; 3]).is_err());
}
#[test]
fn generation_cancellation_never_reactivates_old_work() {
    let mut token = Generation::default();
    assert!(!token.accepts(1));
    let first = token.begin();
    assert!(token.accepts(first));
    token.cancel();
    assert!(!token.accepts(first));
    let second = token.begin();
    assert_ne!(first, second);
    assert!(!token.accepts(first));
}
#[test]
fn walking_search_never_digs_or_crosses_unknown_support() {
    use rusty_mining::{Cell, Grid};
    let mut g = Grid::new([-2, -1, -2], [9, 5, 5]).unwrap();
    for i in 0..g.len() {
        let p = g.position(i);
        g.set(p, if p[1] < 0 { Cell::wall() } else { Cell::air() });
    }
    assert!(walk_route(&g, [0; 3], |p| p == [4, 0, 0], 100).is_some());
    for z in -2..3 {
        for y in 0..3 {
            g.set([2, y, z], Cell::terrain(1));
        }
    }
    assert!(walk_route(&g, [0; 3], |p| p == [4, 0, 0], 100).is_none());
    g.set([0, -1, 0], Cell::unknown());
    assert!(walk_route(&g, [0; 3], |_| true, 100).is_none());
}
#[test]
fn known_partial_states_progress_only_one_item_and_keep_other_properties() {
    let props = |n: &str| BTreeMap::from([("layers".into(), n.into())]);
    assert!(safe_progress(
        "minecraft:snow",
        &props("4"),
        "minecraft:air",
        &BTreeMap::new(),
        "minecraft:snow",
        &props("1")
    ));
    assert!(safe_progress(
        "minecraft:snow",
        &props("4"),
        "minecraft:snow",
        &props("1"),
        "minecraft:snow",
        &props("2")
    ));
    assert!(!safe_progress(
        "minecraft:snow",
        &props("4"),
        "minecraft:snow",
        &props("1"),
        "minecraft:snow",
        &props("3")
    ));
    assert!(!safe_progress(
        "minecraft:snow",
        &props("4"),
        "minecraft:stone",
        &BTreeMap::new(),
        "minecraft:snow",
        &props("1")
    ));
    let double = BTreeMap::from([
        ("type".into(), "double".into()),
        ("waterlogged".into(), "false".into()),
    ]);
    let mut half = double.clone();
    half.insert("type".into(), "bottom".into());
    assert!(safe_progress(
        "minecraft:oak_slab",
        &double,
        "minecraft:air",
        &BTreeMap::new(),
        "minecraft:oak_slab",
        &half
    ));
    half.insert("waterlogged".into(), "true".into());
    assert!(!safe_progress(
        "minecraft:oak_slab",
        &double,
        "minecraft:air",
        &BTreeMap::new(),
        "minecraft:oak_slab",
        &half
    ));
    assert_eq!(material_cost(&props("4")), 4);
}

#[test]
fn ordered_stats_barrier_cannot_accept_old_ui_reply_or_unsolicited_packet() {
    let mut fence = StatsFence::default();
    let old = fence.request().unwrap();
    let swap = fence.request().unwrap();
    assert_eq!(fence.last_request(), swap);
    assert!(fence.reliable());
    assert_eq!(fence.snapshot(), (2, 0, true));
    fence.response();
    assert!(fence.confirmed(old));
    assert!(!fence.confirmed(swap));
    fence.response();
    assert!(fence.confirmed(swap));
    fence.response();
    assert!(!fence.confirmed(swap));
    assert!(fence.request().is_err());
    let new_connection = StatsFence::default();
    assert!(!new_connection.confirmed(swap));
}
#[test]
fn north_stairs_require_the_opposite_side_even_when_already_in_reach() {
    let p = BTreeMap::from([("facing".into(), "north".into())]);
    assert!(!approach_matches(
        "minecraft:oak_stairs",
        &p,
        [0, 0, -2],
        [0, 0, 0]
    ));
    assert!(approach_matches(
        "minecraft:oak_stairs",
        &p,
        [0, 0, 2],
        [0, 0, 0]
    ));
    assert!(!approach_matches(
        "minecraft:oak_stairs",
        &p,
        [3, 0, 1],
        [0, 0, 0]
    ));
}
