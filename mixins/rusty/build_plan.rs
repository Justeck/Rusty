// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
pub const MAX_CELLS: usize = 4096;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BuildSettings {
    pub enabled: bool,
    pub width: u8,
    pub depth: u8,
    pub height: u8,
    pub hollow: bool,
    pub floor: bool,
    pub roof: bool,
    pub schematic_min_health: u8,
    pub schematic_min_food: u8,
}
impl Default for BuildSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            width: 5,
            depth: 5,
            height: 3,
            hollow: true,
            floor: true,
            roof: false,
            schematic_min_health: 12,
            schematic_min_food: 10,
        }
    }
}
impl BuildSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=20).contains(&self.schematic_min_health) || self.schematic_min_food > 20 {
            return Err("Schematic minimum health must be 1–20 and food must be 0–20".into());
        }
        if [self.width, self.depth, self.height]
            .iter()
            .all(|v| (1..=16).contains(v))
        {
            Ok(())
        } else {
            Err("Plan dimensions must each be between 1 and 16 blocks".into())
        }
    }
}
pub fn cells(settings: &BuildSettings, origin: [i32; 3]) -> Result<Vec<[i32; 3]>, String> {
    settings.validate()?;
    let sizes = [settings.width, settings.height, settings.depth];
    let mut high = origin;
    for i in 0..3 {
        high[i] = origin[i]
            .checked_add(i32::from(sizes[i]) - 1)
            .ok_or("Plan coordinate overflow")?;
    }
    if [origin[0], origin[2], high[0], high[2]]
        .iter()
        .any(|v| i64::from(*v).abs() > 29_999_984)
        || origin[1] < -2048
        || high[1] > 2047
    {
        return Err("Plan leaves supported world bounds".into());
    }
    let capacity =
        usize::from(settings.width) * usize::from(settings.height) * usize::from(settings.depth);
    if capacity > MAX_CELLS {
        return Err("Plan exceeds 4096 cells".into());
    }
    let mut out = Vec::with_capacity(capacity);
    for y in 0..settings.height {
        for z in 0..settings.depth {
            for x in 0..settings.width {
                if !settings.hollow
                    || x == 0
                    || x == settings.width - 1
                    || z == 0
                    || z == settings.depth - 1
                    || settings.floor && y == 0
                    || settings.roof && y == settings.height - 1
                {
                    out.push([
                        origin[0] + i32::from(x),
                        origin[1] + i32::from(y),
                        origin[2] + i32::from(z),
                    ]);
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hollow_plan_excludes_the_interior_and_has_deterministic_bottom_up_order() {
        let cfg = BuildSettings {
            width: 3,
            depth: 3,
            height: 3,
            roof: true,
            ..Default::default()
        };
        let plan = cells(&cfg, [-2, 64, -3]).unwrap();
        assert_eq!(plan.len(), 26);
        assert!(!plan.contains(&[-1, 65, -2]));
        assert_eq!(plan[0], [-2, 64, -3]);
        assert_eq!(plan[25], [0, 66, -1]);
        assert_eq!(
            plan.iter()
                .copied()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            26
        );
    }
    #[test]
    fn floor_and_roof_are_independent_and_solid_fills_every_cell() {
        let cfg = BuildSettings {
            width: 3,
            depth: 3,
            height: 3,
            floor: false,
            roof: false,
            ..Default::default()
        };
        assert_eq!(cells(&cfg, [0; 3]).unwrap().len(), 24);
        assert_eq!(
            cells(
                &BuildSettings {
                    floor: true,
                    ..cfg.clone()
                },
                [0; 3]
            )
            .unwrap()
            .len(),
            25
        );
        assert_eq!(
            cells(
                &BuildSettings {
                    hollow: false,
                    ..cfg
                },
                [0; 3]
            )
            .unwrap()
            .len(),
            27
        );
    }
    #[test]
    fn bounded_dimensions_and_world_edges_are_checked_before_allocation() {
        let cfg = BuildSettings {
            width: 16,
            depth: 16,
            height: 16,
            hollow: false,
            ..Default::default()
        };
        assert_eq!(cells(&cfg, [0; 3]).unwrap().len(), MAX_CELLS);
        assert!(cells(&cfg, [i32::MAX, 0, 0]).is_err());
        assert!(cells(&cfg, [29_999_980, 0, 0]).is_err());
        assert!(cells(&BuildSettings { height: 0, ..cfg }, [0; 3]).is_err());
    }
}
