// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Independently implemented read-only utility presentation and bounds.
use serde::{Deserialize, Serialize};

pub const MAX_PREVIEW_SCAN: usize = 64;
pub const MAX_LABEL_CHARS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UtilitySettings {
    pub equipment_hud: bool,
    pub environment_hud: bool,
    pub container_preview: bool,
    pub warning_percent: u8,
    pub preview_rows: u8,
}
impl Default for UtilitySettings {
    fn default() -> Self {
        Self {
            equipment_hud: false,
            environment_hud: false,
            container_preview: false,
            warning_percent: 20,
            preview_rows: 5,
        }
    }
}
impl UtilitySettings {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=50).contains(&self.warning_percent) {
            return Err("Equipment warning percentage must be 1-50".into());
        }
        if !(1..=9).contains(&self.preview_rows) {
            return Err("Container preview rows must be 1-9".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageMaximum {
    Unknown,
    Removed,
    Value(u32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Durability {
    Unknown,
    NotDamageable,
    Unbreakable,
    Remaining { remaining: u32, maximum: u32 },
}
impl Durability {
    pub fn observe(
        damage: u32,
        maximum: DamageMaximum,
        damage_removed: bool,
        unbreakable: bool,
    ) -> Self {
        if unbreakable {
            return Self::Unbreakable;
        }
        if damage_removed {
            return Self::NotDamageable;
        }
        match maximum {
            DamageMaximum::Unknown => Self::Unknown,
            DamageMaximum::Removed | DamageMaximum::Value(0) => Self::NotDamageable,
            DamageMaximum::Value(maximum) => Self::Remaining {
                remaining: maximum.saturating_sub(damage),
                maximum,
            },
        }
    }
    pub fn percentage(self) -> Option<u8> {
        match self {
            Self::Remaining { remaining, maximum } if maximum > 0 => {
                Some((u64::from(remaining.min(maximum)) * 100 / u64::from(maximum)) as u8)
            }
            _ => None,
        }
    }
    pub fn warning(self, threshold: u8) -> bool {
        matches!(self, Self::Remaining { remaining, maximum } if maximum > 0
            && u64::from(remaining) * 100 <= u64::from(maximum) * u64::from(threshold.min(100)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalLight {
    pub block: u8,
    pub sky: u8,
}
pub fn observed_light(
    packed: u8,
    loaded: bool,
    inside_height: bool,
    ready: bool,
) -> Option<LocalLight> {
    (loaded && inside_height && ready).then_some(LocalLight {
        block: packed & 15,
        sky: packed >> 4,
    })
}
pub fn block_position(at: [f64; 3]) -> Option<[i32; 3]> {
    let floor = at.map(f64::floor);
    floor
        .iter()
        .all(|v| v.is_finite() && *v >= f64::from(i32::MIN) && *v <= f64::from(i32::MAX))
        .then(|| floor.map(|v| v as i32))
}
pub fn bounded_label(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() && *c != '§')
        .take(MAX_LABEL_CHARS)
        .collect()
}

pub struct PreviewInput<'a> {
    pub slot: u8,
    pub item: &'a str,
    pub count: u32,
}
#[derive(Debug, PartialEq, Eq)]
pub struct PreviewSelection {
    pub indices: Vec<usize>,
    pub truncated: bool,
}
/// Indices are into a supplied, top-level item component. Never traverses
/// nested containers and never reads a world chest or another player's data.
pub fn select_preview(
    entries: &[PreviewInput<'_>],
    limit: usize,
    source_has_more: bool,
) -> PreviewSelection {
    let mut seen = [false; 256];
    let mut candidates = Vec::with_capacity(entries.len().min(MAX_PREVIEW_SCAN));
    let mut truncated = source_has_more || entries.len() > MAX_PREVIEW_SCAN;
    for (index, entry) in entries.iter().take(MAX_PREVIEW_SCAN).enumerate() {
        if entry.count == 0 || entry.item.is_empty() || entry.item == "minecraft:air" {
            continue;
        }
        if std::mem::replace(&mut seen[usize::from(entry.slot)], true) {
            // Malformed duplicate slots are not counted twice. Preserve the
            // first supplied entry and signal that the view is incomplete.
            truncated = true;
            continue;
        }
        candidates.push((entry.slot, index));
    }
    candidates.sort_unstable_by_key(|(slot, _)| *slot);
    let limit = limit.clamp(1, 9);
    truncated |= candidates.len() > limit;
    PreviewSelection {
        indices: candidates
            .into_iter()
            .take(limit)
            .map(|(_, index)| index)
            .collect(),
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durability_uses_received_maximum_and_keeps_unknown_distinct() {
        let actual = Durability::observe(12, DamageMaximum::Value(100), false, false);
        assert_eq!(
            actual,
            Durability::Remaining {
                remaining: 88,
                maximum: 100
            }
        );
        assert_eq!(actual.percentage(), Some(88));
        assert_eq!(
            Durability::observe(5, DamageMaximum::Unknown, false, false),
            Durability::Unknown
        );
        assert_eq!(
            Durability::observe(5, DamageMaximum::Removed, false, false),
            Durability::NotDamageable
        );
        assert_eq!(
            Durability::observe(5, DamageMaximum::Value(0), false, false),
            Durability::NotDamageable
        );
        assert_eq!(
            Durability::observe(5, DamageMaximum::Value(100), true, false),
            Durability::NotDamageable
        );
        assert_eq!(
            Durability::observe(5, DamageMaximum::Value(100), false, true),
            Durability::Unbreakable
        );
        assert_eq!(Durability::Unknown.percentage(), None);
        assert!(!Durability::Unknown.warning(50));
    }

    #[test]
    fn warnings_do_not_round_a_healthy_item_down_into_the_warning_range() {
        let above = Durability::observe(79, DamageMaximum::Value(101), false, false);
        assert_eq!(above.percentage(), Some(21));
        assert!(!above.warning(21)); // 22/101 is greater than 21%.
        assert!(above.warning(22));
        let huge = Durability::observe(u32::MAX, DamageMaximum::Value(u32::MAX), false, false);
        assert_eq!(huge.percentage(), Some(0));
        assert!(huge.warning(1));
        assert_eq!(
            Durability::observe(500, DamageMaximum::Value(100), false, false).percentage(),
            Some(0)
        );
    }

    #[test]
    fn light_never_turns_missing_or_pending_samples_into_darkness() {
        assert_eq!(
            observed_light(0xE5, true, true, true),
            Some(LocalLight { block: 5, sky: 14 })
        );
        assert_eq!(
            observed_light(0, true, true, true),
            Some(LocalLight { block: 0, sky: 0 })
        );
        for (loaded, inside, ready) in [
            (false, true, true),
            (true, false, true),
            (true, true, false),
        ] {
            assert_eq!(observed_light(0, loaded, inside, ready), None);
        }
    }

    #[test]
    fn negative_positions_floor_and_nonfinite_or_out_of_range_are_unknown() {
        assert_eq!(block_position([-0.1, -63.9, 16.0]), Some([-1, -64, 16]));
        for at in [
            [f64::NAN, 0., 0.],
            [0., f64::INFINITY, 0.],
            [f64::MAX, 0., 0.],
            [0., 0., -2_147_483_649.],
        ] {
            assert_eq!(block_position(at), None);
        }
    }

    #[test]
    fn preview_is_slot_ordered_bounded_and_does_not_expand_nested_data() {
        let entries = [
            PreviewInput {
                slot: 9,
                item: "minecraft:stone",
                count: 64,
            },
            PreviewInput {
                slot: 2,
                item: "minecraft:shulker_box",
                count: 1,
            },
            PreviewInput {
                slot: 4,
                item: "minecraft:diamond",
                count: 3,
            },
            PreviewInput {
                slot: 2,
                item: "minecraft:gold_ingot",
                count: 9,
            },
            PreviewInput {
                slot: 1,
                item: "minecraft:air",
                count: 1,
            },
            PreviewInput {
                slot: 0,
                item: "minecraft:stone",
                count: 0,
            },
        ];
        assert_eq!(
            select_preview(&entries, 2, false),
            PreviewSelection {
                indices: vec![1, 2],
                truncated: true
            }
        );
        assert_eq!(
            select_preview(&[], 9, false),
            PreviewSelection {
                indices: vec![],
                truncated: false
            }
        );
        assert!(select_preview(&entries[..1], 9, true).truncated);
    }

    #[test]
    fn preview_limits_work_even_when_a_server_supplies_many_entries() {
        let entries: Vec<_> = (0..200)
            .map(|slot| PreviewInput {
                slot,
                item: "minecraft:stone",
                count: 1,
            })
            .collect();
        let selection = select_preview(&entries, usize::MAX, false);
        assert_eq!(selection.indices, (0..9).collect::<Vec<_>>());
        assert!(selection.truncated);
    }

    #[test]
    fn untrusted_labels_cannot_inject_lines_or_unbounded_text() {
        assert_eq!(bounded_label("Diamond\nHelmet\r\0§"), "DiamondHelmet");
        let text = "🪨".repeat(200);
        assert_eq!(bounded_label(&text).chars().count(), MAX_LABEL_CHARS);
    }

    #[test]
    fn settings_reject_bounds_and_keep_partial_old_profiles_off_by_default() {
        let mut settings: UtilitySettings = serde_json::from_str("{}").unwrap();
        assert!(
            !settings.equipment_hud && !settings.environment_hud && !settings.container_preview
        );
        assert!(settings.validate().is_ok());
        settings.preview_rows = 0;
        assert!(settings.validate().is_err());
        settings.preview_rows = 9;
        settings.warning_percent = 51;
        assert!(settings.validate().is_err());
        assert!(
            serde_json::from_str::<UtilitySettings>(
                "{\"container_preview\": true, \"preview_rows\": -1}"
            )
            .is_err()
        );
    }
}
