use rusty_horizon::rusty_mining::*;

#[test]
fn protected_partial_objects_inside_the_box_keep_their_support_without_blocking_safe_nearby_standing()
 {
    let area = Region::new([2, 1, 2], [3, 1, 2]).unwrap();
    let mut g = Grid::new([0, 0, 0], [6, 5, 5]).unwrap();
    for i in 0..g.len() {
        let p = g.position(i);
        g.set(p, if p[1] == 0 { Cell::wall() } else { Cell::air() });
    }
    g.set(
        [2, 1, 2],
        area.constrain(Cell::resource(1), [2, 1, 2], Some(1)),
    );
    g.set(
        [3, 1, 2],
        area.constrain(Cell::passive(false), [3, 1, 2], Some(1)),
    );
    assert!(
        !g.can_mine([2, 1, 2], false),
        "Do not drop a protected sign by removing its support"
    );
    assert!(
        g.safe_stand([3, 1, 1]),
        "A protected decoration is not a movement hazard"
    );
}

#[test]
fn a_live_shrunken_border_must_contain_the_entire_block_volume() {
    let inside = Region::new([-4, 1, -4], [3, 2, 3]).unwrap();
    let crossing = Region::new([0, 1, 3], [1, 2, 4]).unwrap();
    assert!(inside.within_border([-4.0, -4.0, 4.0, 4.0]));
    assert!(!crossing.within_border([-4.0, -4.0, 4.0, 4.0]));
    assert!(crossing.within_border([-8.0, -8.0, 8.0, 8.0]));
    assert!(!inside.within_border([f64::NAN, -4.0, 4.0, 4.0]));
}

#[test]
fn a_hazard_anywhere_in_the_selection_prevents_starting_partial_excavation() {
    let mut survey = ClearSurvey::default();
    survey.observe([0, 1, 0], Cell::resource(1));
    survey.observe([8, 1, 8], Cell::fluid());
    assert_eq!(survey.dangerous, 1);
    assert_eq!(survey.highest, Some(1));
    assert!(!survey.complete());
}

#[test]
fn walking_brakes_at_the_target_column_until_a_checked_drop_has_landed() {
    let target = [1.5, 200.0, 4.5];
    assert!(brake_before_landing(
        [1.517, 200.9216, 4.5008],
        target,
        0.15,
        false
    ));
    assert!(brake_before_landing([1.5, 201.0, 4.5], target, 0.15, true));
    assert!(!brake_before_landing(
        [1.738, 200.5358, 4.5115],
        target,
        0.15,
        false
    ));
    assert!(!brake_before_landing([1.5, 200.0, 4.5], target, 0.15, true));
}

#[test]
fn corner_order_is_inclusive_and_negative_coordinates_do_not_change_the_box() {
    let r = Region::new([2, 4, -3], [-1, 2, -5]).unwrap();
    assert_eq!(r.min, [-1, 2, -5]);
    assert_eq!(r.max, [2, 4, -3]);
    assert_eq!(r.volume(), 36);
    assert_eq!(r.position(0), Some([-1, 2, -5]));
    assert_eq!(r.position(35), Some([2, 4, -3]));
    assert_eq!(r.position(36), None);
    assert!(r.contains([2, 4, -3]));
    assert!(!r.contains([3, 4, -3]));
    assert_eq!(r.edges().len(), 12);
    assert!(Region::new([0, 0, 0], [32, 0, 0]).is_err());
    assert!(Region::new([i32::MIN, 0, 0], [i32::MAX, 0, 0]).is_err());
    assert!(Region::new([30_000_000, 0, 0], [30_000_000, 0, 0]).is_err());
    assert!(Region::new([0, 0, 0], [31, 31, 31]).is_ok());
}

#[test]
fn survey_selects_the_highest_eligible_layer_and_never_calls_leftovers_complete() {
    let mut s = ClearSurvey::default();
    s.observe([0, 2, 0], Cell::resource(8));
    s.observe([0, 5, 0], Cell::resource(4));
    s.observe([0, 7, 0], Cell::wall());
    s.observe([0, 9, 0], Cell::unknown());
    s.observe([0, 10, 0], Cell::air());
    assert_eq!(
        (s.remaining, s.protected, s.unknown, s.highest),
        (3, 1, 1, Some(5))
    );
    assert!(!s.complete());
    let mut empty = ClearSurvey::default();
    empty.observe([0, 0, 0], Cell::air());
    assert!(empty.complete());
    empty.observe([0, 0, 1], Cell::fluid());
    assert!(!empty.complete());
}

#[test]
fn filtering_occurs_per_position_and_preserves_floor_and_partial_shape_facts() {
    let region = Region::new([2, 1, 2], [3, 3, 3]).unwrap();
    let rock = Cell::resource(7);
    let mut grid = Grid::new([0, 0, 0], [6, 6, 6]).unwrap();
    for i in 0..grid.len() {
        let p = grid.position(i);
        grid.set(p, Cell::air());
    }
    let inside = region.constrain(rock, [2, 3, 2], Some(3));
    let outside = region.constrain(rock, [1, 3, 2], Some(3));
    let lower = region.constrain(rock, [2, 2, 2], Some(3));
    assert!(inside.is_resource());
    assert!(!outside.is_resource());
    assert!(!lower.is_resource());
    assert!(outside.is_support());
    grid.set([1, 3, 2], outside);
    grid.set([2, 2, 2], lower);
    assert!(!grid.can_mine([1, 3, 2], true));
    assert!(!grid.can_mine([2, 2, 2], true));
    let partial = Cell::resource_shape(1, false);
    assert!(partial.is_resource());
    assert!(!partial.is_support());
    assert!(!region.constrain(partial, [0, 0, 0], Some(3)).is_support());
}

#[test]
fn route_actions_never_excavate_cheaper_identical_material_outside_the_selected_box() {
    let area = Region::new([4, 1, 3], [4, 1, 3]).unwrap();
    let mut g = Grid::new([0, 0, 0], [9, 5, 7]).unwrap();
    for i in 0..g.len() {
        let p = g.position(i);
        let material = if p == [2, 1, 3] || p == [4, 1, 3] {
            Cell::resource(5)
        } else if p[1] == 0 {
            Cell::wall()
        } else {
            Cell::air()
        };
        g.set(p, area.constrain(material, p, Some(1)));
    }
    while !g.prepare(100) {}
    let mut search = Search::new(g, [1, 1, 3], false, 32768).unwrap();
    loop {
        match search.advance(100) {
            SearchResult::Working => {}
            SearchResult::Found(plan) => {
                assert_eq!(plan.target, [4, 1, 3]);
                for a in plan.actions {
                    if let Action::Mine(p, _) = a {
                        assert!(area.contains(p));
                    }
                }
                break;
            }
            result => panic!("Expected a safe detour: {result:?}"),
        }
    }
}

#[test]
fn clearing_safety_preferences_are_independent_and_have_no_activation_or_saved_corners() {
    let s = ClearSettings::default();
    assert!(s.validate().is_ok());
    let json = serde_json::to_value(&s).unwrap();
    assert_eq!(json["min_health"], 12.0);
    assert!(json.get("enabled").is_none());
    assert!(json.get("first").is_none());
    assert!(json.get("second").is_none());
    let runtime = s.mining();
    assert!(!runtime.tunnels);
    assert_eq!(runtime.radius, 32);
}

#[test]
fn dependent_partial_blocks_outside_the_box_protect_their_neighboring_support() {
    let area = Region::new([2, 1, 2], [2, 1, 2]).unwrap();
    let mut g = Grid::new([0, 0, 0], [5, 5, 5]).unwrap();
    for i in 0..g.len() {
        let p = g.position(i);
        g.set(p, Cell::air());
    }
    g.set(
        [2, 1, 2],
        area.constrain(Cell::resource(1), [2, 1, 2], Some(1)),
    );
    g.set(
        [2, 2, 2],
        area.constrain(Cell::resource_shape(1, false), [2, 2, 2], Some(1)),
    );
    assert!(
        !g.can_mine([2, 1, 2], false),
        "Do not remove support beneath an outside attachment or half of a tall block"
    );
}
