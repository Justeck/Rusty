// Local screen-space HUD layout. Copyright 2026 Justeck, Apache-2.0.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MIN_SCALE: f32 = 0.5;
pub const MAX_SCALE: f32 = 2.0;
pub const PANELS: &[&str] = &[
    "status_hud",
    "coordinates",
    "finder",
    "compass",
    "waypoints",
    "equipment_hud",
    "environment_hud",
    "container_preview",
    "build_planner",
    "fps_hud",
    "clicks_hud",
    "portal_coordinates",
    "clock_hud",
    "speed_hud",
    "target_info",
    "light_overlay",
    "trajectories",
    "minimap",
];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Placement {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
}
impl Default for Placement {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            scale: 1.0,
        }
    }
}
impl Placement {
    pub fn validate(self) -> Result<(), String> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !self.scale.is_finite()
            || !(0.0..=1.0).contains(&self.x)
            || !(0.0..=1.0).contains(&self.y)
            || !(MIN_SCALE..=MAX_SCALE).contains(&self.scale)
        {
            return Err("HUD position must be 0-100% and size 50-200%".into());
        }
        Ok(())
    }
    /// Position is the fraction of available space after the scaled panel fits.
    /// The effective scale also fits oversized panels in tiny viewports, without
    /// changing the stored preference, so resize restores the requested size.
    pub fn resolve(self, viewport: [f32; 2], size: [f32; 2]) -> Option<Resolved> {
        self.validate().ok()?;
        if !viewport
            .iter()
            .chain(size.iter())
            .all(|n| n.is_finite() && *n > 0.0)
        {
            return None;
        }
        let margin = 3.0_f32.min(viewport[0] / 4.0).min(viewport[1] / 4.0);
        let room = viewport.map(|n| (n - margin * 2.0).max(f32::EPSILON));
        let scale = self.scale.min(room[0] / size[0]).min(room[1] / size[1]);
        Some(Resolved {
            origin: [
                margin + (room[0] - size[0] * scale).max(0.0) * self.x,
                margin + (room[1] - size[1] * scale).max(0.0) * self.y,
            ],
            scale,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resolved {
    pub origin: [f32; 2],
    pub scale: f32,
}
impl Resolved {
    pub fn point(self, point: [f32; 2], source: [f32; 2]) -> [f32; 2] {
        [
            self.origin[0] + (point[0] - source[0]) * self.scale,
            self.origin[1] + (point[1] - source[1]) * self.scale,
        ]
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HudLayout(pub BTreeMap<String, Placement>);
impl HudLayout {
    /// Keep a default information column separate as its content grows. Explicit
    /// user placements are resolved independently and never moved by this fit.
    pub fn resolve_default_stack(
        &self,
        viewport: [f32; 2],
        panels: &[(&str, [f32; 2])],
    ) -> Vec<Option<Resolved>> {
        self.resolve_default_stack_until(viewport, panels, viewport[1] * 0.87)
    }
    pub fn resolve_default_columns(
        &self,
        viewport: [f32; 2],
        panels: &[(&str, [f32; 2])],
        minimap_top: Option<f32>,
    ) -> Vec<Option<Resolved>> {
        let mut placed: Vec<_> = panels
            .iter()
            .map(|(id, size)| {
                PANELS
                    .contains(id)
                    .then(|| self.get(id).resolve(viewport, *size))
                    .flatten()
            })
            .collect();
        for column in [0.0, 0.5, 1.0] {
            let mut indices: Vec<_> = panels
                .iter()
                .enumerate()
                .filter_map(|(i, (id, _))| {
                    (placed[i].is_some()
                        && !self.0.contains_key(*id)
                        && default_placement(id).x == column)
                        .then_some(i)
                })
                .collect();
            indices.sort_by(|&a, &b| {
                default_placement(panels[a].0)
                    .y
                    .total_cmp(&default_placement(panels[b].0).y)
            });
            let entries: Vec<_> = indices.iter().map(|&i| panels[i]).collect();
            let bottom = if column == 1.0 {
                minimap_top
                    .filter(|v| v.is_finite())
                    .map_or(viewport[1] * 0.87, |y| (y - 3.0).min(viewport[1] * 0.87))
            } else {
                viewport[1] * 0.87
            };
            for (i, resolved) in indices
                .into_iter()
                .zip(self.resolve_default_stack_until(viewport, &entries, bottom))
            {
                placed[i] = resolved;
            }
        }
        placed
    }
    fn resolve_default_stack_until(
        &self,
        viewport: [f32; 2],
        panels: &[(&str, [f32; 2])],
        limit: f32,
    ) -> Vec<Option<Resolved>> {
        let mut placed: Vec<_> = panels
            .iter()
            .map(|(id, size)| self.get(id).resolve(viewport, *size))
            .collect();
        let defaults: Vec<_> = panels
            .iter()
            .enumerate()
            .filter_map(|(i, (id, _))| {
                (!self.0.contains_key(*id) && placed[i].is_some()).then_some(i)
            })
            .collect();
        let Some(&first) = defaults.first() else {
            return placed;
        };
        let margin = 3.0_f32.min(viewport[0] / 4.0).min(viewport[1] / 4.0);
        if limit <= margin {
            for i in defaults {
                placed[i] = None;
            }
            return placed;
        }
        let top = placed[first].unwrap().origin[1]
            .min((limit - panels[first].1[1] * placed[first].unwrap().scale).max(margin));
        let mut next = top;
        for &i in &defaults {
            let panel = placed[i].as_mut().unwrap();
            panel.origin[1] = if i == first {
                top
            } else {
                panel.origin[1].max(next)
            };
            next = panel.origin[1] + panels[i].1[1] * panel.scale + 3.0;
        }
        let bottom = next - 3.0;
        let available = (limit - top).max(f32::EPSILON);
        let fit = (available / (bottom - top)).min(1.0);
        for i in defaults {
            let panel = placed[i].as_mut().unwrap();
            panel.origin[1] = top + (panel.origin[1] - top) * fit;
            panel.scale *= fit;
            panel.origin[0] = margin
                + (viewport[0] - margin * 2.0 - panels[i].1[0] * panel.scale).max(0.0)
                    * self.get(panels[i].0).x;
        }
        placed
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.0.len() > PANELS.len() {
            return Err("Too many HUD panels".into());
        }
        for (id, placement) in &self.0 {
            if !PANELS.contains(&id.as_str()) {
                return Err(format!("Unknown screen HUD panel: {id}"));
            }
            placement.validate()?;
        }
        Ok(())
    }
    pub fn get(&self, id: &str) -> Placement {
        self.0
            .get(id)
            .copied()
            .unwrap_or_else(|| default_placement(id))
    }
    pub fn set(&mut self, id: &str, placement: Placement) -> Result<(), String> {
        if !PANELS.contains(&id) {
            return Err("Unknown screen HUD panel".into());
        }
        placement.validate()?;
        self.0.insert(id.to_owned(), placement);
        Ok(())
    }
    pub fn reset(&mut self, id: &str) {
        self.0.remove(id);
    }
}
pub fn default_placement(id: &str) -> Placement {
    let (x, y) = match id {
        "status_hud" => (0.0, 0.0),
        "coordinates" => (0.0, 0.18),
        "finder" => (0.5, 0.02),
        "compass" => (0.5, 0.1),
        "waypoints" => (0.5, 0.16),
        "equipment_hud" => (1.0, 0.22),
        "environment_hud" => (1.0, 0.5),
        "container_preview" => (1.0, 0.74),
        "build_planner" => (0.0, 0.88),
        "fps_hud" => (0.0, 0.3),
        "clicks_hud" => (0.0, 0.36),
        "portal_coordinates" => (0.0, 0.42),
        "clock_hud" => (0.0, 0.48),
        "speed_hud" => (0.0, 0.54),
        "target_info" => (0.0, 0.6),
        "light_overlay" => (0.0, 0.8),
        "trajectories" => (0.0, 0.84),
        "minimap" => (1.0, 1.0),
        _ => (0.0, 0.0),
    };
    Placement { x, y, scale: 1.0 }
}

/// Stable setting IDs keep queued/dragged changes attached to the original panel.
pub const CONTROLS: &[(&str, [&str; 3])] = &[
    (
        "status_hud",
        [
            "hud:status_hud:x",
            "hud:status_hud:y",
            "hud:status_hud:scale",
        ],
    ),
    (
        "coordinates",
        [
            "hud:coordinates:x",
            "hud:coordinates:y",
            "hud:coordinates:scale",
        ],
    ),
    (
        "finder",
        ["hud:finder:x", "hud:finder:y", "hud:finder:scale"],
    ),
    (
        "compass",
        ["hud:compass:x", "hud:compass:y", "hud:compass:scale"],
    ),
    (
        "waypoints",
        ["hud:waypoints:x", "hud:waypoints:y", "hud:waypoints:scale"],
    ),
    (
        "equipment_hud",
        [
            "hud:equipment_hud:x",
            "hud:equipment_hud:y",
            "hud:equipment_hud:scale",
        ],
    ),
    (
        "environment_hud",
        [
            "hud:environment_hud:x",
            "hud:environment_hud:y",
            "hud:environment_hud:scale",
        ],
    ),
    (
        "container_preview",
        [
            "hud:container_preview:x",
            "hud:container_preview:y",
            "hud:container_preview:scale",
        ],
    ),
    (
        "build_planner",
        [
            "hud:build_planner:x",
            "hud:build_planner:y",
            "hud:build_planner:scale",
        ],
    ),
    (
        "fps_hud",
        ["hud:fps_hud:x", "hud:fps_hud:y", "hud:fps_hud:scale"],
    ),
    (
        "clicks_hud",
        [
            "hud:clicks_hud:x",
            "hud:clicks_hud:y",
            "hud:clicks_hud:scale",
        ],
    ),
    (
        "portal_coordinates",
        [
            "hud:portal_coordinates:x",
            "hud:portal_coordinates:y",
            "hud:portal_coordinates:scale",
        ],
    ),
    (
        "clock_hud",
        ["hud:clock_hud:x", "hud:clock_hud:y", "hud:clock_hud:scale"],
    ),
    (
        "speed_hud",
        ["hud:speed_hud:x", "hud:speed_hud:y", "hud:speed_hud:scale"],
    ),
    (
        "target_info",
        [
            "hud:target_info:x",
            "hud:target_info:y",
            "hud:target_info:scale",
        ],
    ),
    (
        "light_overlay",
        [
            "hud:light_overlay:x",
            "hud:light_overlay:y",
            "hud:light_overlay:scale",
        ],
    ),
    (
        "trajectories",
        [
            "hud:trajectories:x",
            "hud:trajectories:y",
            "hud:trajectories:scale",
        ],
    ),
    (
        "minimap",
        ["hud:minimap:x", "hud:minimap:y", "hud:minimap:scale"],
    ),
];
pub fn control(id: &str) -> Option<(&'static str, usize)> {
    CONTROLS
        .iter()
        .find_map(|(panel, ids)| ids.iter().position(|v| *v == id).map(|axis| (*panel, axis)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lone_default_panel_moves_up_before_reducing_readable_size() {
        let viewport = [800.0, 450.0];
        let panels = [("build_planner", [180.0, 11.0])];
        let resolved =
            HudLayout::default().resolve_default_columns(viewport, &panels, None)[0].unwrap();
        assert_eq!(resolved.scale, 1.0);
        assert!(resolved.origin[1] + 11.0 <= viewport[1] * 0.87 + 0.001);
    }

    #[test]
    fn expanded_right_column_reserves_default_minimap_and_keeps_right_edges() {
        let viewport = [600.0, 337.5];
        let panels = [
            ("equipment_hud", [210.0, 71.0]),
            ("environment_hud", [170.0, 27.0]),
            ("container_preview", [260.0, 126.0]),
        ];
        let placed = HudLayout::default().resolve_default_columns(viewport, &panels, Some(240.0));
        let mut bottom = 0.0;
        for ((_, size), resolved) in panels.iter().zip(placed) {
            let resolved = resolved.unwrap();
            assert!(resolved.origin[1] >= bottom);
            bottom = resolved.origin[1] + size[1] * resolved.scale;
            assert!(bottom <= 237.001);
            assert!((resolved.origin[0] + size[0] * resolved.scale - 597.0).abs() < 0.001);
        }
    }
    #[test]
    fn populated_finder_keeps_compass_and_waypoint_text_separate() {
        let panels = [
            ("finder", [240.0, 159.0]),
            ("compass", [100.0, 16.0]),
            ("waypoints", [200.0, 71.0]),
        ];
        for viewport in [[600.0, 337.5], [320.0, 180.0]] {
            let placed = HudLayout::default().resolve_default_columns(viewport, &panels, None);
            let mut bottom = 0.0;
            for ((_, size), resolved) in panels.iter().zip(placed) {
                let resolved = resolved.unwrap();
                assert!(resolved.origin[1] >= bottom);
                bottom = resolved.origin[1] + size[1] * resolved.scale;
                assert!(bottom <= viewport[1] * 0.87 + 0.001);
                assert!(
                    (resolved.origin[0] + size[0] * resolved.scale / 2.0 - viewport[0] / 2.0).abs()
                        < 0.001
                );
            }
        }
    }
    #[test]
    fn column_order_is_independent_of_draw_order_and_custom_panels_stay_fixed() {
        let viewport = [600.0, 337.5];
        let panels = [
            ("build_planner", [180.0, 49.0]),
            ("clock_hud", [120.0, 16.0]),
            ("status_hud", [180.0, 38.0]),
            ("fps_hud", [140.0, 16.0]),
            ("coordinates", [200.0, 16.0]),
        ];
        let mut layout = HudLayout::default();
        let custom = Placement {
            x: 0.7,
            y: 0.3,
            scale: 1.25,
        };
        layout.set("clock_hud", custom).unwrap();
        let placed = layout.resolve_default_columns(viewport, &panels, None);
        assert_eq!(placed[1], custom.resolve(viewport, panels[1].1));
        let mut bottom = 0.0;
        for i in [2, 4, 3, 0] {
            let resolved = placed[i].unwrap();
            assert!(resolved.origin[1] >= bottom);
            bottom = resolved.origin[1] + panels[i].1[1] * resolved.scale;
        }
        assert!(bottom <= viewport[1] * 0.87 + 0.001);
        assert_eq!(layout.0.len(), 1);
    }

    #[test]
    fn expanded_information_panels_keep_separate_default_bounds() {
        for viewport in [[600.0, 337.5], [800.0, 450.0], [320.0, 180.0]] {
            for target_height in [38.0, 82.0] {
                let panels = [
                    ("fps_hud", [140.0, 16.0]),
                    ("clicks_hud", [100.0, 16.0]),
                    ("portal_coordinates", [140.0, 16.0]),
                    ("clock_hud", [100.0, 16.0]),
                    ("speed_hud", [100.0, 16.0]),
                    ("target_info", [160.0, target_height]),
                    ("light_overlay", [180.0, 16.0]),
                    ("trajectories", [180.0, 16.0]),
                ];
                let layout = HudLayout::default();
                let placed = layout.resolve_default_stack(viewport, &panels);
                let mut bottom = 0.0;
                for ((_, size), resolved) in panels.iter().zip(placed) {
                    let resolved = resolved.unwrap();
                    assert!(resolved.origin[1] >= bottom);
                    bottom = resolved.origin[1] + size[1] * resolved.scale;
                    assert!(bottom <= viewport[1] * 0.87 + 0.001);
                }
                assert!(
                    layout.0.is_empty(),
                    "Automatic fitting must not save preferences"
                );
            }
        }
    }
    #[test]
    fn automatic_information_stack_preserves_explicit_user_placement() {
        let mut layout = HudLayout::default();
        let custom = Placement {
            x: 0.8,
            y: 0.3,
            scale: 1.25,
        };
        layout.set("speed_hud", custom).unwrap();
        let panels = [("speed_hud", [100.0, 16.0]), ("target_info", [160.0, 82.0])];
        let viewport = [600.0, 337.5];
        let placed = layout.resolve_default_stack(viewport, &panels);
        assert_eq!(placed[0], custom.resolve(viewport, panels[0].1));
        assert_eq!(layout.get("speed_hud"), custom);
        assert!(
            layout
                .resolve_default_stack([0.0, 0.0], &panels)
                .iter()
                .all(Option::is_none)
        );
    }
    #[test]
    fn viewport_resize_and_oversize_panels_remain_visible() {
        let requested = Placement {
            x: 1.0,
            y: 1.0,
            scale: 2.0,
        };
        for viewport in [[800.0, 600.0], [140.0, 90.0], [1.0, 1.0]] {
            let resolved = requested.resolve(viewport, [300.0, 200.0]).unwrap();
            let end = resolved.point([300.0, 200.0], [0.0, 0.0]);
            assert!(resolved.origin.iter().all(|n| *n >= 0.0));
            assert!(end[0] <= viewport[0] + 0.001 && end[1] <= viewport[1] + 0.001);
        }
        assert_eq!(requested.scale, 2.0);
        assert_eq!(
            requested
                .resolve([800.0, 600.0], [300.0, 200.0])
                .unwrap()
                .scale,
            2.0
        );
    }
    #[test]
    fn finite_values_and_known_screen_panels_only() {
        let mut layout = HudLayout::default();
        for placement in [
            Placement {
                x: f32::NAN,
                ..Default::default()
            },
            Placement {
                y: f32::INFINITY,
                ..Default::default()
            },
            Placement {
                scale: 0.1,
                ..Default::default()
            },
        ] {
            assert!(layout.set("fps_hud", placement).is_err());
        }
        assert!(layout.set("ore_boxes", Placement::default()).is_err());
        assert!(
            Placement::default()
                .resolve([0.0, 20.0], [10.0, 10.0])
                .is_none()
        );
    }
    #[test]
    fn independent_panels_reset_and_round_trip() {
        let mut layout = HudLayout::default();
        let placement = Placement {
            x: 0.7,
            y: 0.8,
            scale: 1.5,
        };
        layout.set("fps_hud", placement).unwrap();
        assert_eq!(layout.get("fps_hud"), placement);
        assert_eq!(layout.get("compass"), default_placement("compass"));
        let loaded: HudLayout =
            serde_json::from_str(&serde_json::to_string(&layout).unwrap()).unwrap();
        assert_eq!(loaded, layout);
        layout.reset("fps_hud");
        assert_eq!(layout.get("fps_hud"), default_placement("fps_hud"));
    }
}
