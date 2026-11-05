// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Independently implemented ordinary-input assistance. No protocol operations.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AssistanceSettings {
    pub safe_walk: bool,
    pub auto_jump: bool,
    pub auto_tool: bool,
    pub tool_min_remaining: u8,
    pub trigger_bot: bool,
    pub trigger_players: bool,
    pub auto_eat: bool,
    pub eat_below: u8,
    pub auto_fish: bool,
    pub fish_delay: u8,
    pub fish_recast: bool,
    pub auto_totem: bool,
    pub armor_assist: bool,
    pub refill: bool,
}
impl Default for AssistanceSettings {
    fn default() -> Self {
        Self {
            safe_walk: false,
            auto_jump: false,
            auto_tool: false,
            tool_min_remaining: 5,
            trigger_bot: false,
            trigger_players: false,
            auto_eat: false,
            eat_below: 16,
            auto_fish: false,
            fish_delay: 4,
            fish_recast: false,
            auto_totem: false,
            armor_assist: false,
            refill: false,
        }
    }
}
impl AssistanceSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !(2..=20).contains(&self.fish_delay) {
            return Err("Fishing delay must be 2-20 client ticks".into());
        }
        if !(1..=19).contains(&self.eat_below) {
            return Err("Eat threshold must be 1-19 food points".into());
        }
        if self.tool_min_remaining > 100 {
            return Err("Tool minimum remaining durability must be 0-100 percent".into());
        }
        Ok(())
    }
}

pub struct FoodCandidate<'a> {
    pub slot: usize,
    pub id: &'a str,
    pub nutrition: u32,
    pub saturation: f32,
}
pub fn best_food(foods: &[FoodCandidate<'_>], selected: usize) -> Option<usize> {
    let safe = |id: &str| {
        matches!(
            id,
            "minecraft:apple"
                | "minecraft:baked_potato"
                | "minecraft:beetroot"
                | "minecraft:beetroot_soup"
                | "minecraft:bread"
                | "minecraft:carrot"
                | "minecraft:cooked_beef"
                | "minecraft:cooked_chicken"
                | "minecraft:cooked_cod"
                | "minecraft:cooked_mutton"
                | "minecraft:cooked_porkchop"
                | "minecraft:cooked_rabbit"
                | "minecraft:cooked_salmon"
                | "minecraft:cookie"
                | "minecraft:dried_kelp"
                | "minecraft:glow_berries"
                | "minecraft:melon_slice"
                | "minecraft:mushroom_stew"
                | "minecraft:potato"
                | "minecraft:pumpkin_pie"
                | "minecraft:rabbit_stew"
                | "minecraft:sweet_berries"
        )
    };
    let valid = |food: &&FoodCandidate<'_>| {
        food.slot < 9
            && safe(food.id)
            && (1..=20).contains(&food.nutrition)
            && food.saturation.is_finite()
            && (0.0..=40.0).contains(&food.saturation)
    };
    // API359 reads 26.3's absolute saturation points (bread: 6.0), not
    // the legacy nutrition multiplier (bread: 0.6).
    let score = |food: &FoodCandidate<'_>| food.saturation;
    let mut best = foods.iter().filter(valid).find(|f| f.slot == selected);
    for food in foods.iter().filter(valid) {
        if best.is_none_or(|b| {
            score(food) > score(b) || (score(food) == score(b) && food.nutrition > b.nutrition)
        }) {
            best = Some(food);
        }
    }
    best.map(|f| f.slot)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EatProgress {
    Hold,
    Complete,
    Cancel,
}
/// Count changes can confirm consumption; item components and damage identify
/// the food being used and must remain identical throughout ownership.
pub fn food_identity_matches<T: PartialEq>(
    original: (&str, u32, &T),
    current: (&str, u32, &T),
) -> bool {
    original == current
}
pub fn eat_progress(
    owned: usize,
    selected: usize,
    started: bool,
    using: bool,
    consumed: bool,
    interrupted: bool,
    timed_out: bool,
) -> EatProgress {
    if owned != selected || interrupted || timed_out {
        EatProgress::Cancel
    } else if consumed {
        EatProgress::Complete
    } else if started && !using {
        EatProgress::Cancel
    } else {
        EatProgress::Hold
    }
}

pub fn horizontal_direction(forward: f32, strafe: f32, yaw: f32) -> Option<[f64; 2]> {
    if !forward.is_finite() || !strafe.is_finite() || !yaw.is_finite() {
        return None;
    }
    let length = f64::from(forward).hypot(f64::from(strafe));
    if length < 0.01 {
        return None;
    }
    let (sin, cos) = f64::from(yaw).to_radians().sin_cos();
    Some([
        (f64::from(strafe) * cos - f64::from(forward) * sin) / length,
        (f64::from(forward) * cos + f64::from(strafe) * sin) / length,
    ])
}

#[derive(Clone, Copy, Debug)]
pub struct ToolCandidate {
    pub slot: usize,
    pub speed: f32,
    pub correct_drops: bool,
    pub remaining_percent: Option<u8>,
}
pub fn best_tool(tools: &[ToolCandidate], selected: usize, minimum: u8) -> Option<usize> {
    if selected >= 9 {
        return None;
    }
    let eligible = |t: &&ToolCandidate| {
        t.slot < 9
            && t.speed.is_finite()
            && t.speed > 0.
            && t.remaining_percent.is_none_or(|p| p >= minimum)
    };
    let mut best = tools.iter().filter(eligible).find(|t| t.slot == selected);
    for tool in tools.iter().filter(eligible) {
        if best.is_none_or(|b| {
            (tool.correct_drops && !b.correct_drops)
                || (tool.correct_drops == b.correct_drops && tool.speed > b.speed)
        }) {
            best = Some(tool);
        }
    }
    best.filter(|t| t.slot != selected).map(|t| t.slot)
}

#[derive(Default, Debug)]
pub struct SelectionLease {
    previous: Option<usize>,
    owned: Option<usize>,
    interrupted: bool,
}
impl SelectionLease {
    pub fn owns_selection(&self) -> bool {
        self.owned.is_some() && !self.interrupted
    }
    pub fn choose(&mut self, selected: usize, desired: Option<usize>) -> Option<usize> {
        if self.owned.is_some_and(|owned| selected != owned) {
            self.previous = None;
            self.owned = None;
            self.interrupted = true;
        }
        if self.interrupted || selected >= 9 {
            return None;
        }
        let desired = desired.filter(|d| *d < 9 && *d != selected)?;
        self.previous.get_or_insert(selected);
        self.owned = Some(desired);
        Some(desired)
    }
    pub fn stop(&mut self, selected: usize) -> Option<usize> {
        let restore = self.previous.filter(|_| self.owned == Some(selected));
        *self = Self::default();
        restore
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn food_selection_only_accepts_known_safe_food_and_prioritizes_saturation() {
        let input = [
            FoodCandidate {
                slot: 0,
                id: "minecraft:rotten_flesh",
                nutrition: 4,
                saturation: 9.,
            },
            FoodCandidate {
                slot: 1,
                id: "minecraft:bread",
                nutrition: 5,
                saturation: 0.6,
            },
            FoodCandidate {
                slot: 2,
                id: "minecraft:cooked_beef",
                nutrition: 8,
                saturation: 0.8,
            },
            FoodCandidate {
                slot: 3,
                id: "minecraft:enchanted_golden_apple",
                nutrition: 4,
                saturation: 99.,
            },
            FoodCandidate {
                slot: 4,
                id: "custom:food",
                nutrition: 100,
                saturation: 100.,
            },
        ];
        assert_eq!(best_food(&input, 0), Some(2));
        assert_eq!(
            best_food(
                &[
                    FoodCandidate {
                        slot: 9,
                        id: "minecraft:bread",
                        nutrition: 5,
                        saturation: 0.6
                    },
                    FoodCandidate {
                        slot: 0,
                        id: "minecraft:bread",
                        nutrition: 5,
                        saturation: f32::NAN
                    }
                ],
                0
            ),
            None
        );
    }
    #[test]
    fn eating_waits_for_ack_and_cancels_on_slot_change_interruption_and_timeout() {
        assert_eq!(
            eat_progress(2, 2, false, false, false, false, false),
            EatProgress::Hold
        );
        assert_eq!(
            eat_progress(2, 2, true, true, false, false, false),
            EatProgress::Hold
        );
        assert_eq!(
            eat_progress(2, 2, true, false, true, false, false),
            EatProgress::Complete
        );
        assert_eq!(
            eat_progress(2, 2, true, false, false, false, false),
            EatProgress::Cancel
        );
        assert_eq!(
            eat_progress(2, 3, true, true, false, false, false),
            EatProgress::Cancel
        );
        assert_eq!(
            eat_progress(2, 2, true, true, false, true, false),
            EatProgress::Cancel
        );
        assert_eq!(
            eat_progress(2, 2, true, true, false, false, true),
            EatProgress::Cancel
        );
    }
    #[test]
    fn movement_matches_minecraft_yaw_and_normalizes_diagonal() {
        assert_eq!(horizontal_direction(1., 0., 0.), Some([0., 1.]));
        let west = horizontal_direction(1., 0., 90.).unwrap();
        assert!((west[0] + 1.).abs() < 1e-6 && west[1].abs() < 1e-6);
        let diagonal = horizontal_direction(1., 1., 0.).unwrap();
        assert!((diagonal[0].hypot(diagonal[1]) - 1.).abs() < 1e-6);
        assert!(diagonal[0] > 0. && diagonal[1] > 0.);
        assert!(horizontal_direction(0., 0., 0.).is_none());
        assert!(horizontal_direction(f32::NAN, 1., 0.).is_none());
    }
    fn tool(
        slot: usize,
        speed: f32,
        correct_drops: bool,
        remaining_percent: Option<u8>,
    ) -> ToolCandidate {
        ToolCandidate {
            slot,
            speed,
            correct_drops,
            remaining_percent,
        }
    }
    #[test]
    fn tool_selection_prioritizes_drops_then_speed_and_keeps_ties() {
        let tools = [
            tool(0, 9., false, Some(100)),
            tool(1, 4., true, Some(50)),
            tool(2, 4., true, Some(100)),
        ];
        assert_eq!(best_tool(&tools, 0, 5), Some(1));
        assert_eq!(best_tool(&tools, 2, 5), None);
        assert_eq!(
            best_tool(&[tool(0, 1., true, None), tool(1, 1., true, None)], 0, 5),
            None
        );
    }
    #[test]
    fn tool_selection_rejects_worn_unknown_and_invalid_candidates() {
        let tools = [
            tool(0, 1., false, None),
            tool(1, 10., true, Some(4)),
            tool(2, f32::NAN, true, Some(100)),
            tool(12, 100., true, Some(100)),
        ];
        assert_eq!(best_tool(&tools, 0, 5), None);
        assert_eq!(
            best_tool(
                &[tool(0, 1., false, None), tool(1, 2., true, Some(5))],
                0,
                5
            ),
            Some(1)
        );
    }
    #[test]
    fn slot_lease_restores_only_owned_selection_and_yields_to_manual_changes() {
        let mut lease = SelectionLease::default();
        assert_eq!(lease.choose(2, Some(4)), Some(4));
        assert_eq!(lease.choose(4, Some(5)), Some(5));
        assert_eq!(lease.stop(5), Some(2));
        assert_eq!(lease.stop(2), None);
        assert_eq!(lease.choose(2, Some(4)), Some(4));
        assert_eq!(lease.choose(6, Some(4)), None);
        assert_eq!(lease.choose(6, Some(5)), None);
        assert_eq!(lease.stop(6), None);
        assert_eq!(lease.choose(6, Some(4)), Some(4));
    }
    #[test]
    fn old_profiles_are_disabled_and_invalid_bounds_fail() {
        let settings: AssistanceSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, AssistanceSettings::default());
        assert!(
            AssistanceSettings {
                tool_min_remaining: 101,
                ..settings
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn player_targeting_is_an_explicit_persisted_opt_in() {
        let explicit = serde_json::from_str::<AssistanceSettings>(r#"{"trigger_players":true}"#);
        assert!(explicit.is_ok());
        assert!(
            serde_json::to_string(&explicit.unwrap())
                .unwrap()
                .contains(r#""trigger_players":true"#)
        );
    }
    #[test]
    fn same_item_id_with_replaced_food_components_loses_ownership() {
        let original = serde_json::json!({"custom_name":"Lunch"});
        let replacement =
            serde_json::json!({"custom_name":"Lunch","minecraft:food":{"nutrition":1}});
        assert!(food_identity_matches(
            ("minecraft:bread", 0, &original),
            ("minecraft:bread", 0, &original)
        ));
        assert!(!food_identity_matches(
            ("minecraft:bread", 0, &original),
            ("minecraft:bread", 0, &replacement)
        ));
        assert!(!food_identity_matches(
            ("minecraft:bread", 0, &original),
            ("minecraft:bread", 1, &original)
        ));
    }
    #[test]
    fn native_food_saturation_is_absolute_points_not_the_legacy_modifier() {
        // Prepared 26.3 items.json: bread is nutrition 5, saturation 6.0.
        assert_eq!(
            best_food(
                &[FoodCandidate {
                    slot: 0,
                    id: "minecraft:bread",
                    nutrition: 5,
                    saturation: 6.0
                }],
                0
            ),
            Some(0)
        );
    }
}
