use rusty_horizon::rusty_mining::*;

fn room() -> Grid {
    let mut g = Grid::new([0, 0, 0], [9, 5, 7]).unwrap();
    for y in 0..5 {
        for z in 0..7 {
            for x in 0..9 {
                g.set([x, y, z], if y == 0 { Cell::wall() } else { Cell::air() });
            }
        }
    }
    g
}
fn solve(mut g: Grid, dig: bool) -> SearchResult {
    while !g.prepare(17) {}
    let mut search = Search::new(g, [1, 1, 3], dig, 32768).unwrap();
    loop {
        let out = search.advance(11);
        if !matches!(out, SearchResult::Working) {
            return out;
        }
    }
}
fn plan(g: Grid, dig: bool) -> Plan {
    match solve(g, dig) {
        SearchResult::Found(p) => p,
        x => panic!("Expected route, got {x:?}"),
    }
}

#[test]
fn selects_resource_presets_and_validates_custom_ids_without_enabling_a_session() {
    let mut s = Settings::default();
    assert!(s.matches("minecraft:diamond_ore"));
    assert!(s.matches("minecraft:deepslate_diamond_ore"));
    assert!(!s.matches("minecraft:gold_ore"));
    s.resource = 11;
    s.custom_block = "example:rare_ore".into();
    assert!(s.validate().is_ok());
    assert!(s.matches("example:rare_ore"));
    s.custom_block = "../secret".into();
    assert!(s.validate().is_err());
    s = Settings::default();
    s.radius = 100;
    assert!(s.validate().is_err());
}
#[test]
fn cheapest_route_uses_walk_detour_instead_of_slow_tunnelling() {
    let mut g = room();
    g.set([6, 1, 3], Cell::resource(40));
    g.set([3, 1, 3], Cell::terrain(300));
    g.set([3, 2, 3], Cell::terrain(300));
    let p = plan(g, true);
    assert_eq!(p.target, [6, 1, 3]);
    assert!(
        p.actions
            .iter()
            .all(|a| !matches!(a, Action::Mine([3, _, 3], _)))
    );
    assert!(p.estimated_ticks < 100);
}
#[test]
fn fastest_estimated_route_digs_cheap_safe_obstruction_head_first() {
    let mut g = room();
    g.set([6, 1, 3], Cell::resource(40));
    g.set([3, 1, 3], Cell::terrain(1));
    g.set([3, 2, 3], Cell::terrain(1));
    let p = plan(g, true);
    let head = p
        .actions
        .iter()
        .position(|a| *a == Action::Mine([3, 2, 3], false))
        .unwrap();
    let feet = p
        .actions
        .iter()
        .position(|a| *a == Action::Mine([3, 1, 3], false))
        .unwrap();
    assert!(head < feet);
    assert!(p.estimated_ticks < 70);
}
#[test]
fn resource_distance_alone_does_not_choose_a_slow_expensive_target() {
    let mut g = room();
    g.set([3, 1, 3], Cell::resource(500));
    g.set([6, 1, 3], Cell::resource(20));
    assert_eq!(plan(g, true).target, [6, 1, 3]);
}
#[test]
fn disabling_access_tunnels_still_mines_exposed_targets() {
    let mut g = room();
    g.set([6, 1, 3], Cell::resource(40));
    let p = plan(g, false);
    assert!(
        p.actions
            .iter()
            .all(|a| !matches!(a, Action::Mine(_, false)))
    );
    assert!(p.actions.contains(&Action::Mine([6, 1, 3], true)));
}
#[test]
fn refuses_fluid_or_falling_material_exposed_by_mining() {
    for unsafe_cell in [Cell::fluid(), Cell::falling(), Cell::unknown()] {
        let mut g = room();
        g.set([5, 1, 3], Cell::resource(20));
        g.set([5, 2, 3], unsafe_cell);
        assert!(!matches!(solve(g, true), SearchResult::Found(_)));
    }
}
#[test]
fn never_walks_into_unknown_ground_or_uses_a_removed_support() {
    let mut g = room();
    g.set([6, 1, 3], Cell::resource(20));
    for z in 0..7 {
        for y in 0..5 {
            g.set([4, y, z], Cell::unknown());
        }
    }
    assert!(!matches!(solve(g, true), SearchResult::Found(_)));
}
#[test]
fn bounded_search_never_turns_a_partial_route_into_mining() {
    let mut g = room();
    g.set([7, 1, 5], Cell::resource(30));
    while !g.prepare(17) {}
    let mut s = Search::new(g, [1, 1, 3], true, 1).unwrap();
    assert!(matches!(s.advance(100), SearchResult::Limited));
    assert!(Grid::new([0, 0, 0], [65, 512, 65]).is_err());
}
#[test]
fn pure_guard_stops_on_manual_input_damage_and_unknown_session() {
    let s = Settings::default();
    let c = Context::default();
    assert_eq!(guard(&s, &c), None);
    for why in 0..8 {
        let mut c = Context::default();
        match why {
            0 => c.manual = true,
            1 => c.damaged = true,
            2 => c.same_world = false,
            3 => c.health = 2.0,
            4 => c.food = 0,
            5 => c.inventory_space = false,
            6 => c.focused = false,
            _ => c.playing = false,
        }
        assert!(guard(&s, &c).is_some(), "Unsafe context {why} continued");
    }
}

#[test]
fn safe_walk_requires_clear_supported_cells_and_refuses_adjacent_lava() {
    let mut g = room();
    assert!(g.can_walk([1, 1, 3], [2, 1, 3]));
    g.set([2, 0, 3], Cell::air());
    assert!(!g.can_walk([1, 1, 3], [2, 1, 3]));
    g.set([2, 0, 3], Cell::wall());
    g.set([2, 1, 2], Cell::fluid());
    assert!(!g.can_walk([1, 1, 3], [2, 1, 3]));
    assert!(!g.can_walk([1, 1, 3], [3, 1, 3]));
}

#[test]
fn clearances_and_stairs_do_not_allow_vertical_shaft_mining() {
    let mut g = room();
    g.set([1, 0, 3], Cell::resource(1));
    g.set([1, 3, 3], Cell::resource(1));
    // Resources can be approached from the side, but no plan may break under its current stand.
    if let SearchResult::Found(p) = solve(g, true) {
        let mut stand = [1, 1, 3];
        for a in p.actions {
            match a {
                Action::Walk(p) => stand = p,
                Action::Mine(p, _) => assert!(p[0] != stand[0] || p[2] != stand[2]),
            }
        }
    }
    let mut g = room();
    g.set([2, 1, 3], Cell::wall());
    assert!(g.can_walk([1, 1, 3], [2, 2, 3]));
    g.set([1, 3, 3], Cell::wall());
    assert!(!g.can_walk([1, 1, 3], [2, 2, 3]));
}

#[test]
fn settings_round_trip_contains_no_saved_activation_and_defaults_restore_individually() {
    use rusty_horizon::core::Config;
    let mut cfg = Config::default();
    cfg.mining.resource = 1;
    cfg.mining.goal = 37;
    let data = cfg.to_json().unwrap();
    assert!(!data.contains("\"auto_mine\":true"));
    let mut copy = Config::from_json(&data).unwrap();
    assert_eq!(copy.mining, cfg.mining);
    copy.restore_default_path("/mining/goal").unwrap();
    assert_eq!(copy.mining.goal, 16);
    assert_eq!(copy.mining.resource, 1);
}

#[test]
fn no_received_resource_does_not_spend_the_expansion_budget() {
    let mut grid = room();
    while !grid.prepare(23) {}
    let mut search = Search::new(grid, [1, 1, 3], true, 1).unwrap();
    assert!(matches!(search.advance(1), SearchResult::NoPath));
}

#[test]
fn collecting_drops_uses_only_supported_confirmed_openings() {
    let mut grid = room();
    grid.set([2, 1, 3], Cell::resource(20));
    assert_eq!(grid.collection_step([1, 1, 3], [2, 1, 3]), None);
    grid.set([2, 1, 3], Cell::air());
    assert_eq!(grid.collection_step([1, 1, 3], [2, 1, 3]), Some([2, 1, 3]));
    grid.set([2, 0, 3], Cell::air());
    assert_eq!(grid.collection_step([1, 1, 3], [2, 1, 3]), None);
    grid.set([2, 0, 3], Cell::wall());
    assert_eq!(grid.collection_step([1, 1, 3], [2, 2, 3]), Some([2, 1, 3]));
    grid.set([2, 1, 3], Cell::unknown());
    assert_eq!(grid.collection_step([1, 1, 3], [2, 1, 3]), None);
}

#[test]
fn collection_does_not_move_onto_a_remaining_descending_clearance() {
    use std::collections::VecDeque;
    let remaining = VecDeque::from([Action::Mine([2, 0, 3], false), Action::Walk([2, 0, 3])]);
    let mut actions = remaining.clone();
    assert!(!collect_after(&mut actions, Some([2, 1, 3]), false));
    assert_eq!(actions, remaining);
    assert!(collect_after(&mut actions, Some([2, 1, 3]), true));
    assert_eq!(actions, VecDeque::from([Action::Walk([2, 1, 3])]));
    assert!(collect_after(&mut actions, Some([2, 1, 3]), false));
    assert_eq!(actions.len(), 1);
    actions.clear();
    assert!(collect_after(&mut actions, Some([2, 1, 3]), false));
}
