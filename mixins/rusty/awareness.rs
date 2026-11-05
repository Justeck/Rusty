// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AwarenessSettings {
    pub chunk_borders: bool,
    pub light_overlay: bool,
    pub storage_esp: bool,
    pub trajectories: bool,
    pub target_info: bool,
    pub portal_coordinates: bool,
    pub speed_hud: bool,
    pub clock_hud: bool,
    pub entity_tracers: bool,
    pub minimap: bool,
    pub minimap_rotate: bool,
    pub minimap_round: bool,
    pub minimap_size: u32,
    pub minimap_scale: f32,
    pub fps_hud: bool,
    pub clicks_hud: bool,
    pub overlay_radius: u32,
    pub trajectory_steps: u32,
}
impl Default for AwarenessSettings {
    fn default() -> Self {
        Self {
            chunk_borders: false,
            light_overlay: false,
            storage_esp: false,
            trajectories: false,
            target_info: false,
            portal_coordinates: false,
            speed_hud: false,
            clock_hud: false,
            entity_tracers: false,
            minimap: false,
            minimap_rotate: false,
            minimap_round: true,
            minimap_size: 96,
            minimap_scale: 1.,
            fps_hud: false,
            clicks_hud: false,
            overlay_radius: 12,
            trajectory_steps: 100,
        }
    }
}
impl AwarenessSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !(64..=160).contains(&self.minimap_size)
            || !self.minimap_scale.is_finite()
            || !(0.5..=4.).contains(&self.minimap_scale)
        {
            return Err(
                "Minimap size must be 64-160 pixels and scale 0.5-4 blocks per pixel".into(),
            );
        }
        if !(4..=24).contains(&self.overlay_radius) || !(20..=200).contains(&self.trajectory_steps)
        {
            return Err("Awareness radius must be 4-24 and trajectory steps 20-200".into());
        }
        Ok(())
    }
}
pub fn chunk_origin(coordinate: f64) -> Option<i32> {
    (coordinate.is_finite() && coordinate.abs() <= 30_000_000.)
        .then(|| (coordinate.floor() as i32).div_euclid(16) * 16)
}
pub fn portal_position(dimension: &str, at: [f64; 3]) -> Option<(&'static str, [f64; 3])> {
    let (label, scale) = match dimension {
        "minecraft:overworld" => ("Nether", 0.125),
        "minecraft:the_nether" => ("Overworld", 8.),
        _ => return None,
    };
    let projected = [at[0] * scale, at[1], at[2] * scale];
    (at.iter()
        .chain(projected.iter())
        .all(|x| x.is_finite() && x.abs() <= 30_000_000.))
    .then_some((label, projected))
}
pub fn clock_text(ticks: i64) -> String {
    let minutes = ((ticks.rem_euclid(24000) + 6000) % 24000) * 60 / 1000;
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_chunk_edges_use_floor() {
        assert_eq!(chunk_origin(-0.1), Some(-16));
        assert_eq!(chunk_origin(-16.), Some(-16));
        assert_eq!(chunk_origin(16.), Some(16));
        assert_eq!(chunk_origin(f64::NAN), None);
    }
    #[test]
    fn portal_projection_is_dimension_specific_and_bounded() {
        assert_eq!(
            portal_position("minecraft:overworld", [-80., 64., 160.]),
            Some(("Nether", [-10., 64., 20.]))
        );
        assert_eq!(
            portal_position("minecraft:the_nether", [-10., 64., 20.]),
            Some(("Overworld", [-80., 64., 160.]))
        );
        assert_eq!(portal_position("minecraft:the_end", [0.; 3]), None);
        assert_eq!(
            portal_position("minecraft:the_nether", [4_000_000., 64., 0.]),
            None
        );
    }
    #[test]
    fn clock_wraps_negative_and_full_days() {
        assert_eq!(clock_text(0), "06:00");
        assert_eq!(clock_text(6000), "12:00");
        assert_eq!(clock_text(18000), "00:00");
        assert_eq!(clock_text(-1), "05:59");
        assert_eq!(clock_text(24000), "06:00");
    }
    #[test]
    fn defaults_off_limits_and_roundtrip() {
        let d = AwarenessSettings::default();
        assert_eq!(serde_json::from_str::<AwarenessSettings>("{}").unwrap(), d);
        assert!(!d.storage_esp && !d.trajectories && !d.entity_tracers);
        assert!(d.validate().is_ok());
        assert!(
            AwarenessSettings {
                overlay_radius: 25,
                ..d.clone()
            }
            .validate()
            .is_err()
        );
        assert!(
            AwarenessSettings {
                trajectory_steps: 201,
                ..d
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn minimap_limits_reject_invalid_gpu_geometry() {
        let mut cfg = AwarenessSettings {
            minimap_size: 2000,
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
        cfg.minimap_size = 96;
        cfg.minimap_scale = f32::NAN;
        assert!(cfg.validate().is_err());
        cfg.minimap_scale = 0.25;
        assert!(cfg.validate().is_err());
        cfg.minimap_scale = 4.;
        assert!(cfg.validate().is_ok());
    }
}
