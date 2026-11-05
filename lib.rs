#[path = "mixins/rusty/runtime.rs"]
pub mod core;

#[path = "mixins/rusty/freecam.rs"]
pub mod freecam;

#[path = "mixins/rusty/ui_controls.rs"]
pub mod ui_controls;

#[path = "mixins/rusty/catalog.rs"]
pub mod rusty_catalog;

#[path = "mixins/rusty/navigation.rs"]
pub mod rusty_navigation;

#[path = "mixins/rusty/utility.rs"]
pub mod rusty_utility;

#[path = "mixins/rusty/clickgui_model.rs"]
pub mod rusty_clickgui_model;

#[path = "mixins/rusty/world_identity.rs"]
pub mod rusty_world_identity;

#[path = "mixins/rusty/assistance.rs"]
pub mod rusty_assistance;

#[path = "mixins/rusty/inventory_assistance.rs"]
pub mod rusty_inventory_core;

#[path = "mixins/rusty/awareness.rs"]
pub mod rusty_awareness;

#[path = "mixins/rusty/prediction_core.rs"]
pub mod rusty_prediction_core;

#[path = "mixins/rusty/build_plan.rs"]
pub mod rusty_build_plan;

#[cfg(test)]
mod upgrade_profile_tests {
    use super::{core::Config, rusty_catalog::Theme};

    #[test]
    fn displayed_native_version_matches_release_identity() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("horizon-native.json")).unwrap();
        assert_eq!(super::core::VERSION, env!("CARGO_PKG_VERSION"));
        assert_eq!(manifest["version"].as_str(), Some(super::core::VERSION));
    }

    #[test]
    fn legacy_profiles_gain_disabled_navigation_and_default_menu() {
        let legacy = Config::from_json(r#"{"schema":1,"fullbright":true}"#).unwrap();
        assert!(legacy.fullbright);
        assert_eq!(legacy.client.theme, Theme::Classic);
        assert!(!legacy.navigation.compass_enabled);
        assert!(!legacy.navigation.waypoint_hud);
        assert!(!legacy.navigation.breadcrumbs_enabled);
        assert_eq!(
            Config::from_json(&legacy.to_json().unwrap()).unwrap(),
            legacy
        );
    }

    #[test]
    fn nested_profile_validation_rejects_bad_navigation_and_unknown_favorites() {
        assert!(Config::from_json(r#"{"navigation":{"sample_distance":0.0}}"#).is_err());
        assert!(Config::from_json(r#"{"client":{"favorites":["fake_module"]}}"#).is_err());
        let mut cfg = Config::default();
        cfg.client.toggle_favorite("xray");
        cfg.client.theme = Theme::Copper;
        cfg.navigation.compass_enabled = true;
        assert_eq!(Config::from_json(&cfg.to_json().unwrap()).unwrap(), cfg);
    }

    #[test]
    fn feature_profile_import_preserves_local_menu_preferences() {
        let mut current = Config::default();
        current.client.theme = Theme::Copper;
        current.client.toggle_favorite("xray");
        current.client.visit("finder");
        let imported = current
            .import_profile(r#"{"fullbright":true,"client":{"theme":"graphite"}}"#)
            .unwrap();
        assert!(imported.fullbright);
        assert_eq!(imported.client, current.client);
        assert!(current.import_profile(r#"{"scan_radius":0}"#).is_err());
        assert_eq!(current.client.theme, Theme::Copper);
    }

    #[test]
    fn oversized_valid_json_is_rejected_before_profile_deserialization() {
        let padded = " ".repeat(4 * 1024 * 1024) + "{}";
        assert!(Config::from_json(&padded).is_err());
    }
    #[test]
    fn profile_readers_stop_at_the_size_bound_and_preserve_menu_preferences() {
        use std::{cell::Cell, io::Read, rc::Rc};
        struct Oversized(Rc<Cell<usize>>);
        impl Read for Oversized {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                self.0.set(self.0.get() + buffer.len());
                assert!(self.0.get() <= super::core::MAX_PROFILE_BYTES + 1);
                buffer.fill(b' ');
                Ok(buffer.len())
            }
        }
        let bytes = Rc::new(Cell::new(0));
        assert!(Config::read_json(Oversized(bytes.clone())).is_err());
        assert_eq!(bytes.get(), super::core::MAX_PROFILE_BYTES + 1);
        let mut current = Config::default();
        current.client.theme = Theme::Copper;
        let imported = current
            .import_profile_reader(std::io::Cursor::new(b"{\"fullbright\":true}"))
            .unwrap();
        assert!(imported.fullbright);
        assert_eq!(imported.client, current.client);
    }
}

#[cfg(test)]
mod navigation_tests {
    use super::rusty_navigation::*;
    fn scope(server: &str, dimension: &str) -> Scope {
        Scope::new(server, dimension).unwrap()
    }
    #[test]
    fn negative_coordinates_and_minecraft_bearings() {
        let west = direction([-10., -64., -20.], [-20., -64., -20.], 0.).unwrap();
        assert_eq!(west.distance, 10.);
        assert_eq!(west.bearing, 90.);
        assert_eq!(west.relative, 90.);
        assert_eq!(compass(90.).unwrap(), "W");
        assert_eq!(compass(-180.).unwrap(), "N");
        assert_eq!(compass(360.).unwrap(), "S");
        assert_eq!(
            direction([0.; 3], [0., 0., -10.], 170.).unwrap().relative,
            10.
        );
    }
    #[test]
    fn scopes_and_selected_ids_survive_restart_without_leaking() {
        let a = scope("server:a:25565", "minecraft:overworld");
        let b = scope("server:b:25565", "minecraft:overworld");
        let nether = scope("server:a:25565", "minecraft:the_nether");
        let mut data = NavigationData::default();
        let id = data.add(&a, " Base\n§k/../ ", [-1.5, 64., -2.]).unwrap();
        assert!(data.waypoints(&b).is_empty());
        assert!(data.waypoints(&nether).is_empty());
        data.select(&a, Some(id)).unwrap();
        let loaded = NavigationData::from_json(&data.to_json().unwrap()).unwrap();
        assert_eq!(loaded.selected(&a).unwrap().position, [-1.5, 64., -2.]);
        assert!(!loaded.selected(&a).unwrap().name.contains(['\n', '§', '/']));
        assert!(loaded.selected(&b).is_none());
        assert!(loaded.clone().select(&b, Some(id)).is_err());
    }
    #[test]
    fn waypoint_limit_duplicates_and_unknown_selection_are_rejected() {
        let s = scope("local:world", "minecraft:overworld");
        let mut d = NavigationData::default();
        for _ in 0..MAX_WAYPOINTS {
            d.add(&s, "home", [0., 64., 0.]).unwrap();
        }
        assert!(d.add(&s, "overflow", [0., 64., 0.]).is_err());
        let mut raw: serde_json::Value = serde_json::from_str(&d.to_json().unwrap()).unwrap();
        raw["scopes"][0]["waypoints"][1]["id"] = raw["scopes"][0]["waypoints"][0]["id"].clone();
        assert!(NavigationData::from_json(&raw.to_string()).is_err());
        assert!(d.select(&s, Some(u64::MAX)).is_err());
    }
    #[test]
    fn removing_selected_clears_selection_and_coordinate_copy_is_local_text() {
        let s = scope("server:a", "minecraft:overworld");
        let mut d = NavigationData::default();
        let id = d.add(&s, "hill", [-12.5, 70., -0.25]).unwrap();
        d.select(&s, Some(id)).unwrap();
        assert_eq!(d.coordinates(&s, id).unwrap(), "-12.50 70.00 -0.25");
        d.remove(&s, id).unwrap();
        assert!(d.selected(&s).is_none());
        let replacement = d.add(&s, "replacement", [0.; 3]).unwrap();
        assert_ne!(
            replacement, id,
            "deleted ids must not be reused by queued UI actions"
        );
    }
    #[test]
    fn nonfinite_and_world_border_positions_never_enter_state() {
        let mut d = NavigationData::default();
        let s = scope("server:a", "minecraft:overworld");
        for p in [
            [f64::NAN, 0., 0.],
            [0., f64::INFINITY, 0.],
            [WORLD_BORDER + 1., 0., 0.],
        ] {
            assert!(d.add(&s, "bad", p).is_err());
            assert!(direction([0.; 3], p, 0.).is_err());
        }
        assert!(direction([0.; 3], [1.; 3], f64::NAN).is_err());
        assert!(NavigationData::from_json("{\"version\":1,\"scopes\":[],\"unknown\":1}").is_err());
        assert!(NavigationData::from_json(&" ".repeat(MAX_JSON_BYTES + 1)).is_err());
    }
    #[test]
    fn breadcrumbs_only_sample_observed_loaded_positions_and_reset_on_teleport() {
        let s = scope("server:a", "minecraft:overworld");
        let mut t = BreadcrumbTrail::default();
        let settings = NavigationSettings {
            breadcrumbs_enabled: true,
            ..Default::default()
        };
        t.observe(&s, [0., 64., 0.], true, &settings).unwrap();
        t.observe(&s, [0.1, 64., 0.], true, &settings).unwrap();
        assert_eq!(t.points().len(), 1);
        t.observe(&s, [3., 64., 0.], true, &settings).unwrap();
        assert_eq!(t.segments().len(), 1);
        t.observe(&s, [500., 64., 0.], true, &settings).unwrap();
        assert_eq!(t.points().len(), 1);
        assert!(t.segments().is_empty());
        t.observe(&s, [503., 64., 0.], false, &settings).unwrap();
        assert!(t.points().is_empty());
        t.observe(&s, [506., 64., 0.], true, &settings).unwrap();
        assert_eq!(t.points().len(), 1);
        t.observe(
            &scope("server:b", "minecraft:overworld"),
            [508., 64., 0.],
            true,
            &settings,
        )
        .unwrap();
        assert_eq!(t.points().len(), 1);
    }
    #[test]
    fn breadcrumbs_cap_reset_and_defaults_off() {
        let s = scope("local:world", "minecraft:overworld");
        let mut t = BreadcrumbTrail::default();
        let settings = NavigationSettings {
            breadcrumbs_enabled: true,
            ..Default::default()
        };
        for i in 0..700 {
            t.observe(&s, [i as f64 * 3., 64., 0.], true, &settings)
                .unwrap();
        }
        assert_eq!(t.points().len(), MAX_BREADCRUMBS);
        assert_eq!(t.points().back().copied(), Some([2097., 64., 0.]));
        t.observe(&s, [2100., 64., 0.], true, &NavigationSettings::default())
            .unwrap();
        assert!(t.points().is_empty());
        let defaults: NavigationSettings = serde_json::from_str("{}").unwrap();
        assert!(
            !defaults.compass_enabled && !defaults.waypoint_hud && !defaults.breadcrumbs_enabled
        );
        let bad = NavigationSettings {
            sample_distance: f64::NAN,
            ..Default::default()
        };
        assert!(bad.validate().is_err());
    }
    #[test]
    fn atomic_save_preserves_last_valid_data_on_rejected_replacement() {
        let dir = std::env::temp_dir().join(format!("rusty-navigation-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let s = scope("local:test", "minecraft:overworld");
        let mut d = NavigationData::default();
        d.add(&s, "one", [0.; 3]).unwrap();
        d.save(&dir).unwrap();
        let before = std::fs::read(dir.join("navigation.json")).unwrap();
        let mut bad = d.clone();
        bad.scopes[0].waypoints[0].position[0] = f64::NAN;
        assert!(bad.save(&dir).is_err());
        assert_eq!(std::fs::read(dir.join("navigation.json")).unwrap(), before);
        d.add(&s, "two", [1.; 3]).unwrap();
        d.save(&dir).unwrap();
        assert_eq!(NavigationData::load(&dir).unwrap().waypoints(&s).len(), 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[path = "mixins/rusty/waypoint_walk_state.rs"]
pub mod rusty_waypoint_walk_state;

#[path = "mixins/rusty/automation.rs"]
pub mod rusty_automation;

#[path = "mixins/rusty/scan_cache.rs"]
pub mod rusty_scan_cache;

#[cfg(test)]
mod minecraft_ui_tests {
    use super::{core::Config, rusty_catalog::ClientPreferences};
    #[test]
    fn menu_scale_migrates_and_rejects_invalid_settings() {
        let value = serde_json::to_value(ClientPreferences::default()).unwrap();
        assert!((value["menu_scale"].as_f64().unwrap() - 0.7).abs() < 0.0001);
        for scale in [0.0, 0.34, 2.11, 100.0] {
            assert!(
                serde_json::from_value::<ClientPreferences>(
                    serde_json::json!({"menu_scale":scale})
                )
                .is_err()
            );
        }
    }
    #[test]
    fn hud_layout_validation_rejects_offscreen_and_unknown_panels() {
        for layout in [
            serde_json::json!({"fps_hud":{"x":-0.01,"y":0.0,"scale":1.0}}),
            serde_json::json!({"fps_hud":{"x":1.01,"y":0.0,"scale":1.0}}),
            serde_json::json!({"fps_hud":{"x":0.0,"y":0.0,"scale":5.0}}),
            serde_json::json!({"world_ore_boxes":{"x":0.0,"y":0.0,"scale":1.0}}),
        ] {
            assert!(
                Config::from_json(&serde_json::json!({"client":{"hud_layout":layout}}).to_string())
                    .is_err()
            );
        }
    }
    #[test]
    fn local_menu_size_and_hud_positions_survive_feature_import() {
        let current = Config::from_json(r#"{"client":{"menu_scale":1.25,"hud_layout":{"fps_hud":{"x":0.8,"y":0.6,"scale":1.5}}}}"#).unwrap();
        let imported = current
            .import_profile(r#"{"fullbright":true,"client":{"menu_scale":0.75}}"#)
            .unwrap();
        assert!(imported.fullbright);
        let value = serde_json::to_value(imported).unwrap();
        assert_eq!(value["client"]["menu_scale"], 1.25);
        assert!(
            (value["client"]["hud_layout"]["fps_hud"]["x"]
                .as_f64()
                .unwrap()
                - 0.8)
                .abs()
                < 1e-6
        );
    }
}
#[path = "mixins/rusty/hud_layout.rs"]
pub mod rusty_hud_layout;
