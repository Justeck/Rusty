// Rusty native module browser. Copyright 2026 Justeck, Apache-2.0.
use super::{Game, rusty_control, rusty_ui};
use horizon_client::{clipboard::Clipboard, hud::HudBatch, mod_list};
use horizon_client_render::mesher::{
    rusty::{self, ActivationMode, Config},
    rusty_freecam,
};
use horizon_client_render::mesher::{
    rusty_catalog::{self as catalog, Category, Collection, SettingKind, Theme},
    rusty_clickgui_model::*,
    rusty_hud_layout as hud_layout,
};
use std::{cell::RefCell, time::Instant};

struct Menu {
    ui: Model,
    save: SaveState<Config>,
    last_save: Instant,
    opened_at: Instant,
}
impl Default for Menu {
    fn default() -> Self {
        Self {
            ui: Model::default(),
            save: SaveState::new(Config::default()),
            last_save: Instant::now(),
            opened_at: Instant::now(),
        }
    }
}
thread_local! {static MENU:RefCell<Menu>=RefCell::new(Menu::default());}

pub(super) fn is_open() -> bool {
    MENU.with_borrow(|m| m.ui.open)
}
/// Read-only QA state. Rectangles are the actual clipped, visible input targets.
pub(super) fn snapshot() -> serde_json::Value {
    MENU.with_borrow(|m| {
        serde_json::json!({
            "open": m.ui.open,
            "query": m.ui.query.value,
            "selected": m.ui.selected,
            "category": m.ui.category.map(|c|c.id()),
            "collection": format!("{:?}",m.ui.collection),
            "theme": format!("{:?}",m.save.value.client.theme),
            "reduced_motion":m.save.value.client.reduced_motion,
            "search_focus":m.ui.search_focus,
            "capture":m.ui.capture,
            "dirty":m.save.dirty,
            "save_blocked":m.save.blocked,
            "setting_values":catalog::module(m.ui.selected).map(|module| module.settings.iter().map(|s|{
                let value=match s.kind {
                    SettingKind::Bool{..}=>serde_json::json!(flag(&m.save.value,s.id)),
                    SettingKind::Integer{..}|SettingKind::Decimal{..}=>serde_json::json!(number(&m.save.value,s.id)),
                    SettingKind::Choice{..}=>serde_json::json!(m.save.value.activation),
                    SettingKind::Action=>serde_json::Value::Null,
                };
                (s.id.to_owned(),value)
            }).collect::<serde_json::Map<String,serde_json::Value>>()).unwrap_or_default(),
            "ui_error":m.ui.error,
            "status":m.ui.status,
            "local_values":hud_layout::CONTROLS.iter().find(|(id,_)|*id==m.ui.selected).map(|(_,ids)|ids.iter().map(|id|(id.to_string(),serde_json::json!(number(&m.save.value,id)))).collect::<serde_json::Map<String,serde_json::Value>>()).unwrap_or_default(),
            "menu_scale":m.save.value.client.menu_scale,
            "menu_percent":catalog::menu_percent(m.save.value.client.menu_scale),
            "effective_menu_scale":m.ui.render_scale,
            "hud_layout":m.save.value.client.hud_layout,
            "list_rect":m.ui.list_rect.map(|v|v*m.ui.render_scale),
            "detail_rect":m.ui.detail_rect.map(|v|v*m.ui.render_scale),
            "list_scroll":m.ui.list_scroll,
            "detail_scroll":m.ui.detail_scroll,
            "hits":m.ui.hits.iter().take(256).map(|h|serde_json::json!({"rect":h.rect.map(|v|v*m.ui.render_scale),"action":format!("{:?}",h.action)})).collect::<Vec<_>>()
        })
    })
}
pub(super) fn open(game: &mut Game<'_>) {
    if !game.native_window() || !game.native_on(rusty::ID) {
        return;
    }
    if game.using.is_some() {
        game.native_chat("Finish using the held item before opening Rusty.");
        return;
    }
    if !game.native_mod_screen_open(rusty::ID) {
        return;
    }
    // Upload once per opening, never once per rendered frame.
    game.native_mod_image(64, 64, super::rusty_brand::RGBA);
    MENU.with_borrow_mut(|m| {
        let pending = m.save.dirty;
        m.ui = Model {
            open: true,
            status: if pending {
                "Unsaved edits restored. Close or edit to retry saving.".into()
            } else {
                "Search modules or settings. Changes save automatically.".into()
            },
            error: m.save.blocked,
            ..Default::default()
        };
        if !pending {
            m.save = SaveState::new((*rusty::config()).clone());
        }
        m.opened_at = Instant::now();
    });
}

pub(super) fn config_bool<'a>(c: &'a mut Config, id: &str) -> Option<&'a mut bool> {
    Some(match id {
        "minimap" => &mut c.awareness.minimap,
        "minimap_rotate" => &mut c.awareness.minimap_rotate,
        "minimap_round" => &mut c.awareness.minimap_round,
        "fps_hud" => &mut c.awareness.fps_hud,
        "clicks_hud" => &mut c.awareness.clicks_hud,
        "chunk_borders" => &mut c.awareness.chunk_borders,
        "light_overlay" => &mut c.awareness.light_overlay,
        "storage_esp" => &mut c.awareness.storage_esp,
        "trajectories" => &mut c.awareness.trajectories,
        "target_info" => &mut c.awareness.target_info,
        "portal_coordinates" => &mut c.awareness.portal_coordinates,
        "speed_hud" => &mut c.awareness.speed_hud,
        "clock_hud" => &mut c.awareness.clock_hud,
        "entity_tracers" => &mut c.awareness.entity_tracers,
        "build_planner" => &mut c.build.enabled,
        "build_hollow" => &mut c.build.hollow,
        "build_floor" => &mut c.build.floor,
        "build_roof" => &mut c.build.roof,
        "auto_eat" => &mut c.assistance.auto_eat,
        "auto_fish" => &mut c.assistance.auto_fish,
        "fish_recast" => &mut c.assistance.fish_recast,
        "auto_totem" => &mut c.assistance.auto_totem,
        "armor_assist" => &mut c.assistance.armor_assist,
        "refill" => &mut c.assistance.refill,
        "safe_walk" => &mut c.assistance.safe_walk,
        "auto_jump" => &mut c.assistance.auto_jump,
        "auto_tool" => &mut c.assistance.auto_tool,
        "trigger_players" => &mut c.assistance.trigger_players,
        "trigger_bot" => &mut c.assistance.trigger_bot,
        "context_outlines" => &mut c.context_outlines,
        "block_outlines" => &mut c.block_outlines,
        "tracers" => &mut c.tracers,
        "finder" | "finder_enabled" => &mut c.finder_enabled,
        "auto_scan" => &mut c.auto_scan,
        "hud_counts" => &mut c.hud_counts,
        "entity_esp" => &mut c.entity_esp,
        "esp_players" => &mut c.esp_players,
        "esp_mobs" => &mut c.esp_mobs,
        "esp_items" => &mut c.esp_items,
        "fullbright" => &mut c.fullbright,
        "no_fog" => &mut c.no_fog,
        "clear_weather" => &mut c.clear_weather,
        "coordinates" => &mut c.coords_hud,
        "auto_sprint" => &mut c.auto_sprint,
        "no_hurt_camera" => &mut c.no_hurt_camera,
        "freecam_noclip" => &mut c.freecam_noclip,
        "status_hud" => &mut c.hud,
        "compass" | "compass_enabled" => &mut c.navigation.compass_enabled,
        "waypoints" | "waypoint_hud" => &mut c.navigation.waypoint_hud,
        "breadcrumbs" | "breadcrumbs_enabled" => &mut c.navigation.breadcrumbs_enabled,
        "death_markers" => &mut c.navigation.death_markers,
        "equipment_hud" => &mut c.utilities.equipment_hud,
        "environment_hud" => &mut c.utilities.environment_hud,
        "container_preview" => &mut c.utilities.container_preview,
        _ => return None,
    })
}
fn flag(c: &Config, id: &str) -> Option<bool> {
    Some(match id {
        "minimap" => c.awareness.minimap,
        "minimap_rotate" => c.awareness.minimap_rotate,
        "minimap_round" => c.awareness.minimap_round,
        "fps_hud" => c.awareness.fps_hud,
        "clicks_hud" => c.awareness.clicks_hud,
        "chunk_borders" => c.awareness.chunk_borders,
        "light_overlay" => c.awareness.light_overlay,
        "storage_esp" => c.awareness.storage_esp,
        "trajectories" => c.awareness.trajectories,
        "target_info" => c.awareness.target_info,
        "portal_coordinates" => c.awareness.portal_coordinates,
        "speed_hud" => c.awareness.speed_hud,
        "clock_hud" => c.awareness.clock_hud,
        "entity_tracers" => c.awareness.entity_tracers,
        "build_planner" => c.build.enabled,
        "build_hollow" => c.build.hollow,
        "build_floor" => c.build.floor,
        "build_roof" => c.build.roof,
        "auto_eat" => c.assistance.auto_eat,
        "auto_fish" => c.assistance.auto_fish,
        "fish_recast" => c.assistance.fish_recast,
        "auto_totem" => c.assistance.auto_totem,
        "armor_assist" => c.assistance.armor_assist,
        "refill" => c.assistance.refill,
        "safe_walk" => c.assistance.safe_walk,
        "auto_jump" => c.assistance.auto_jump,
        "auto_tool" => c.assistance.auto_tool,
        "trigger_players" => c.assistance.trigger_players,
        "trigger_bot" => c.assistance.trigger_bot,
        "context_outlines" => c.context_outlines,
        "block_outlines" => c.block_outlines,
        "tracers" => c.tracers,
        "finder" | "finder_enabled" => c.finder_enabled,
        "auto_scan" => c.auto_scan,
        "hud_counts" => c.hud_counts,
        "entity_esp" => c.entity_esp,
        "esp_players" => c.esp_players,
        "esp_mobs" => c.esp_mobs,
        "esp_items" => c.esp_items,
        "fullbright" => c.fullbright,
        "no_fog" => c.no_fog,
        "clear_weather" => c.clear_weather,
        "coordinates" => c.coords_hud,
        "auto_sprint" => c.auto_sprint,
        "no_hurt_camera" => c.no_hurt_camera,
        "freecam_noclip" => c.freecam_noclip,
        "status_hud" => c.hud,
        "compass" | "compass_enabled" => c.navigation.compass_enabled,
        "waypoints" | "waypoint_hud" => c.navigation.waypoint_hud,
        "breadcrumbs" | "breadcrumbs_enabled" => c.navigation.breadcrumbs_enabled,
        "death_markers" => c.navigation.death_markers,
        "equipment_hud" => c.utilities.equipment_hud,
        "environment_hud" => c.utilities.environment_hud,
        "container_preview" => c.utilities.container_preview,
        _ => return None,
    })
}

fn enabled(id: &str) -> bool {
    match id {
        "xray" => rusty::active(),
        "freecam" => rusty_freecam::active(),
        "waypoint_walk" => super::rusty_waypoint_walk::active(),
        "zoom" => rusty_control::zoom_active(),
        _ => flag(&rusty::config(), id).unwrap_or(false),
    }
}

fn number(c: &Config, id: &str) -> f64 {
    if id == "menu_scale" {
        return c.client.menu_scale as f64;
    }
    if let Some((panel, axis)) = hud_layout::control(id) {
        let p = c.client.hud_layout.get(panel);
        return match axis {
            0 => p.x as f64 * 100.0,
            1 => p.y as f64 * 100.0,
            _ => p.scale as f64,
        };
    }
    match id {
        "context_radius" => c.context_radius as f64,
        "context_opacity" => c.context_opacity as f64,
        "texture_visibility" => c.texture_visibility() as f64,
        "tracer_limit" => c.tracer_limit as f64,
        "line_width" => c.line_width as f64,
        "scan_radius" => c.scan_radius as f64,
        "scan_interval" => c.scan_interval as f64,
        "max_targets" => c.max_targets as f64,
        "zoom_factor" => c.zoom_factor as f64,
        "freecam_speed" => c.freecam_speed,
        "sample_distance" => c.navigation.sample_distance,
        "teleport_distance" => c.navigation.teleport_distance,
        "warning_percent" => c.utilities.warning_percent as f64,
        "preview_rows" => c.utilities.preview_rows as f64,
        "minimap_size" => c.awareness.minimap_size as f64,
        "minimap_scale" => c.awareness.minimap_scale as f64,
        "overlay_radius" => c.awareness.overlay_radius as f64,
        "trajectory_steps" => c.awareness.trajectory_steps as f64,
        "tool_min_remaining" => c.assistance.tool_min_remaining as f64,
        "build_width" => c.build.width as f64,
        "build_depth" => c.build.depth as f64,
        "build_height" => c.build.height as f64,
        "eat_below" => c.assistance.eat_below as f64,
        "fish_delay" => c.assistance.fish_delay as f64,
        _ => 0.0,
    }
}
fn setting_kind(id: &str) -> Option<SettingKind> {
    if id == "menu_scale" {
        return Some(SettingKind::Decimal {
            min: 0.35,
            max: 2.1,
            step: 0.035,
            default: 0.7,
        });
    }
    if let Some((panel, axis)) = hud_layout::control(id) {
        let default = hud_layout::default_placement(panel);
        return Some(if axis == 2 {
            SettingKind::Decimal {
                min: 0.5,
                max: 2.0,
                step: 0.05,
                default: 1.0,
            }
        } else {
            SettingKind::Decimal {
                min: 0.0,
                max: 100.0,
                step: 1.0,
                default: if axis == 0 {
                    default.x as f64 * 100.0
                } else {
                    default.y as f64 * 100.0
                },
            }
        });
    }
    catalog::MODULES
        .iter()
        .flat_map(|m| m.settings)
        .find(|s| s.id == id)
        .map(|s| s.kind)
}
fn set_number(c: &mut Config, id: &str, value: f64) -> Result<(), String> {
    let kind = setting_kind(id).ok_or("Unknown numeric setting")?;
    let (min, max) = match kind {
        SettingKind::Integer { min, max, .. } => (min as f64, max as f64),
        SettingKind::Decimal { min, max, .. } => (min, max),
        _ => return Err("Setting is not numeric".into()),
    };
    let value = numeric_value(kind, (value - min) / (max - min)).ok_or("Invalid numeric value")?;
    if id == "menu_scale" {
        c.client.menu_scale = value as f32;
        return Ok(());
    }
    if let Some((panel, axis)) = hud_layout::control(id) {
        let mut p = c.client.hud_layout.get(panel);
        match axis {
            0 => p.x = value as f32 / 100.0,
            1 => p.y = value as f32 / 100.0,
            _ => p.scale = value as f32,
        };
        return c.client.hud_layout.set(panel, p);
    }
    match id {
        "context_radius" => c.context_radius = value as u32,
        "context_opacity" => c.context_opacity = value as u8,
        "texture_visibility" => c.set_texture_visibility(value as f32)?,
        "tracer_limit" => c.tracer_limit = value as u32,
        "line_width" => c.line_width = value as f32,
        "scan_radius" => c.scan_radius = value as u32,
        "scan_interval" => c.scan_interval = value as u32,
        "max_targets" => c.max_targets = value as u32,
        "zoom_factor" => c.zoom_factor = value as f32,
        "freecam_speed" => c.freecam_speed = value,
        "sample_distance" => c.navigation.sample_distance = value,
        "teleport_distance" => c.navigation.teleport_distance = value,
        "warning_percent" => c.utilities.warning_percent = value as u8,
        "preview_rows" => c.utilities.preview_rows = value as u8,
        "minimap_size" => c.awareness.minimap_size = value as u32,
        "minimap_scale" => c.awareness.minimap_scale = value as f32,
        "overlay_radius" => c.awareness.overlay_radius = value as u32,
        "trajectory_steps" => c.awareness.trajectory_steps = value as u32,
        "tool_min_remaining" => c.assistance.tool_min_remaining = value as u8,
        "build_width" => c.build.width = value as u8,
        "build_depth" => c.build.depth = value as u8,
        "build_height" => c.build.height = value as u8,
        "eat_below" => c.assistance.eat_below = value as u8,
        "fish_delay" => c.assistance.fish_delay = value as u8,
        _ => return Err("Unknown numeric setting".into()),
    }
    Ok(())
}

fn nudge_number(c: &mut Config, id: &str, steps: f64) -> Result<(), String> {
    let kind = setting_kind(id).ok_or("Unknown numeric setting")?;
    let value = nudge_value(kind, number(c, id), steps).ok_or("Invalid numeric adjustment")?;
    set_number(c, id, value)
}

// No RefCell borrow spans Game calls: native screen transitions can dispatch hooks.
pub(super) fn frame(game: &mut Game<'_>) {
    if game.native_mod_screen() == Some(rusty::ID)
        && (!game.native_on(rusty::ID) || game.player.is_none())
    {
        rusty_control::close_menu(game);
    }
    let owned = game.native_mod_screen() == Some(rusty::ID) && game.native_on(rusty::ID);
    let pending = MENU.with_borrow_mut(|m| {
        let external = !owned && m.ui.open;
        if !m.ui.open {
            return None;
        }
        if !external && !game.focused {
            m.ui.dragging = None;
            m.ui.capture = None;
            m.ui.intents.clear();
            return None;
        }
        if !m.save.dirty {
            m.save = SaveState::new((*rusty::config()).clone());
        }
        let mut intents = m.ui.intents.drain(..).collect::<Vec<_>>();
        if external {
            intents.retain(Intent::persists_after_external_close);
        }
        let closing = m.ui.close_requested || external;
        if external {
            m.ui.deactivate();
        }
        Some((intents, m.save.clone(), m.last_save, closing, external))
    });
    let Some((intents, mut save, last_save, closing, external)) = pending else {
        return;
    };
    let previous = save.clone();
    let requested_retry = closing || !intents.is_empty();
    if requested_retry {
        save.retry();
    }
    let mut cfg = save.value.clone();
    let mut dirty = save.dirty;
    let mut force = closing;
    let mut operations = Vec::new();
    let mut error = None;
    let mut status = None;
    for intent in intents {
        let result = match intent {
            Intent::Numeric(id, value) => {
                dirty = true;
                set_number(&mut cfg, id, value)
            }
            Intent::Nudge(id, steps) => {
                dirty = true;
                nudge_number(&mut cfg, id, steps)
            }
            Intent::ToggleBool(id) => {
                force = true;
                dirty = true;
                if let Some(v) = config_bool(&mut cfg, id) {
                    *v = !*v;
                    Ok(())
                } else {
                    Err("Unsupported toggle".into())
                }
            }
            Intent::ToggleModule(id) => {
                force = true;
                cfg.client.visit(id);
                dirty = true;
                match id {
                    "xray" if cfg.activation == ActivationMode::Hold => {
                        status=Some("X-ray uses Hold activation. Hold its bound key while playing, or choose Toggle.".into());
                        Ok(())
                    }
                    "xray" | "freecam" | "waypoint_walk" => {
                        operations.push(Intent::ToggleModule(id));
                        Ok(())
                    }
                    "zoom" => {
                        status=Some("Zoom is active while its key is held; adjust its factor or binding here.".into());
                        Ok(())
                    }
                    _ => {
                        if let Some(v) = config_bool(&mut cfg, id) {
                            *v = !*v;
                            Ok(())
                        } else {
                            Err("Unsupported module".into())
                        }
                    }
                }
            }
            Intent::Activation => {
                force = true;
                dirty = true;
                cfg.activation = match cfg.activation {
                    ActivationMode::Toggle => ActivationMode::Hold,
                    ActivationMode::Hold => ActivationMode::Toggle,
                };
                Ok(())
            }
            Intent::Favorite(id) => {
                cfg.client.toggle_favorite(id);
                dirty = true;
                force = true;
                Ok(())
            }
            Intent::Visit(id) => {
                dirty |= cfg.client.visit(id);
                Ok(())
            }
            Intent::ResetHud(id) => {
                cfg.client.hud_layout.reset(id);
                dirty = true;
                force = true;
                Ok(())
            }
            Intent::Theme => {
                dirty = true;
                force = true;
                cfg.client.theme = match cfg.client.theme {
                    Theme::Classic => Theme::Graphite,
                    Theme::Graphite => Theme::Copper,
                    Theme::Copper => Theme::Classic,
                };
                Ok(())
            }
            Intent::Motion => {
                dirty = true;
                force = true;
                cfg.client.reduced_motion = !cfg.client.reduced_motion;
                Ok(())
            }
            Intent::Paste => {
                let text = game.clipboard.get();
                MENU.with_borrow_mut(|m| {
                    m.ui.query.insert(&text);
                    m.ui.query_changed();
                });
                Ok(())
            }
            Intent::Copy(text) => {
                game.clipboard.set(&text);
                Ok(())
            }
            Intent::Flush => {
                force = true;
                Ok(())
            }
            other => {
                force = true;
                operations.push(other);
                Ok(())
            }
        };
        if let Err(e) = result {
            error = Some(e);
            break;
        }
    }

    if error.is_none() && dirty {
        match cfg.validated() {
            Ok(valid) => {
                if requested_retry {
                    save.stage(valid);
                } else {
                    save.value = valid;
                    save.dirty = true;
                }
            }
            Err(e) => {
                error = Some(e);
                save = previous;
            }
        }
    }
    if error.is_none() && save.should_save(last_save.elapsed().as_secs_f32() * 1000.0, force) {
        match rusty_control::apply_config(game, save.value.clone()) {
            Ok(()) => {
                save.committed();
                status.get_or_insert("Saved".into());
            }
            Err(e) => {
                save.failed();
                error = Some(e);
            }
        }
    }
    if error.is_none() && !save.dirty {
        for operation in operations {
            match operation {
                Intent::ToggleModule("xray") => rusty_control::toggle_xray(game),
                Intent::ToggleModule("freecam") => rusty_control::toggle_freecam(game),
                Intent::ToggleModule("waypoint_walk") => {
                    rusty_control::close_menu(game);
                    if let Err(e) = super::rusty_waypoint_walk::toggle(game) {
                        game.native_chat(&e);
                        error = Some(e);
                    }
                }
                Intent::BuildAnchor | Intent::BuildPlace => {
                    rusty_control::close_menu(game);
                    let result = if operation == Intent::BuildAnchor {
                        super::rusty_builder::anchor(game)
                    } else {
                        super::rusty_builder::place_next(game)
                    };
                    if let Err(e) = result {
                        game.native_chat(&e);
                        error = Some(e);
                    }
                }
                Intent::Advanced => {
                    rusty_control::close_menu(game);
                    rusty_ui::open(game, true);
                }
                Intent::AddWaypoint => {
                    match super::rusty_navigation_native::add_current(game, "Waypoint") {
                        Ok(id) => {
                            status = Some(format!(
                                "Waypoint {id} added. Use /rusty waypoint list to manage marks."
                            ))
                        }
                        Err(e) => error = Some(e),
                    }
                }
                Intent::Bind(name, value) => match save_binding(game, name, &value) {
                    Ok(()) => {
                        status = Some(if name == "menu" && value == clear_binding() {
                            "Menu key cleared. Rebind in Options > Mods > Rusty, or use /rusty menu.".into()
                        } else {
                            format!("{} binding saved", name)
                        })
                    }
                    Err(e) => error = Some(e),
                },
                _ => {}
            }
        }
        if closing && !external && error.is_none() {
            rusty_control::close_menu(game);
        }
    }

    if error.is_some() && save.dirty {
        save.failed();
    }
    let still_owned = game.native_mod_screen() == Some(rusty::ID);
    MENU.with_borrow_mut(|m| {
        m.save = save;
        if !m.save.dirty {
            m.last_save = Instant::now();
        }
        if !still_owned {
            m.ui.deactivate();
        }
        m.ui.close_requested = false;
        if let Some(e) = error {
            m.ui.status = if m.save.dirty {
                format!("Not saved; edits kept. Retry Close or edit. {e}")
            } else {
                e
            };
            m.ui.error = true;
            m.ui.dragging = None;
        } else if let Some(s) = status {
            m.ui.status = s;
            m.ui.error = false;
        }
    });
}

fn save_binding(game: &mut Game<'_>, name: &str, value: &str) -> Result<(), String> {
    if !(name == "menu" || catalog::MODULES.iter().any(|m| m.keybind_id == Some(name)))
        || value.len() > 96
        || !(value.starts_with("key.keyboard.") || value.starts_with("key.mouse."))
        || value.chars().any(char::is_control)
    {
        return Err("Unsupported key binding".into());
    }
    let mut mods = game.settings.mods.clone();
    mods.keys
        .entry(rusty::ID.into())
        .or_default()
        .insert(name.into(), value.into());
    let path = Game::options_path();
    let dir = path.parent().ok_or("Settings folder is unavailable")?;
    let tmp = dir.join("horizon-mods.rusty.tmp");
    std::fs::write(&tmp, mods.serialize())
        .and_then(|()| std::fs::rename(&tmp, dir.join(mod_list::FILE)))
        .map_err(|e| format!("Could not save binding: {e}"))?;
    game.settings.mods = mods;
    Ok(())
}

fn act(m: &mut Menu, action: Action, at: [f32; 2], right: bool) {
    let ui = &mut m.ui;
    ui.search_focus = false;
    ui.keyboard_hit = None;
    match action {
        Action::Search => {
            ui.search_focus = true;
            ui.query.move_to(ui.query.value.chars().count(), false);
        }
        Action::Select(id) => {
            ui.selected = id;
            ui.detail_scroll = 0.0;
            ui.reveal_setting = true;
            ui.enqueue(Intent::Visit(id));
        }
        Action::Toggle(id) => {
            ui.selected = id;
            ui.enqueue(Intent::ToggleModule(id));
        }
        Action::Favorite(id) => {
            ui.enqueue(Intent::Favorite(id));
        }
        Action::Collection(c) => {
            ui.collection = c;
            ui.query_changed();
        }
        Action::CategoryPicker => ui.categories_open = !ui.categories_open,
        Action::Category(c) => {
            ui.category = c;
            ui.categories_open = false;
            ui.query_changed();
        }
        Action::Theme => {
            ui.enqueue(Intent::Theme);
        }
        Action::Motion => {
            ui.enqueue(Intent::Motion);
        }
        Action::Bool(id) => {
            ui.enqueue(Intent::ToggleBool(id));
        }
        Action::Choice => {
            ui.enqueue(Intent::Activation);
        }
        Action::Numeric(id, rect, kind) => {
            if right {
                let value = match kind {
                    SettingKind::Integer { default, .. } => default as f64,
                    SettingKind::Decimal { default, .. } => default,
                    _ => 0.0,
                };
                ui.enqueue(Intent::Numeric(id, value));
                ui.enqueue(Intent::Flush);
            } else {
                ui.dragging = Some((id, rect, kind));
                if let Some(v) = numeric_value(kind, f64::from((at[0] - rect[0]) / rect[2])) {
                    ui.enqueue(Intent::Numeric(id, v));
                }
            }
        }
        Action::Step(id, steps) => {
            ui.enqueue(Intent::Nudge(id, steps));
            ui.enqueue(Intent::Flush);
        }
        Action::ClearBind(name) => {
            ui.clear_binding(name);
            if name == "menu" {
                ui.status =
                    "Menu key cleared. Open Rusty from Options > Mods to bind it again.".into();
            }
        }
        Action::ResetMenuScale => {
            ui.dragging = None;
            ui.enqueue(Intent::Numeric("menu_scale", catalog::DEFAULT_MENU_SCALE as f64));
            ui.enqueue(Intent::Flush);
        }
        Action::ResetHud(id) => {
            ui.enqueue(Intent::ResetHud(id));
            ui.enqueue(Intent::Flush);
        }
        Action::Bind(name) => {
            if right {
                let default = if name == "menu" {
                    Some("key.keyboard.right.shift")
                } else {
                    catalog::MODULES
                        .iter()
                        .find(|s| s.keybind_id == Some(name))
                        .and_then(|s| s.default_key)
                };
                if let Some(default) = default {
                    ui.enqueue(Intent::Bind(name, default.into()));
                }
            } else {
                ui.capture = Some(name);
                ui.status =
                    "Press a key or mouse button. Escape cancels. Clear unbinds; right-click restores its default."
                        .into();
                ui.error = false;
            }
        }
        Action::BuildAnchor => {
            ui.enqueue(Intent::BuildAnchor);
        }
        Action::BuildPlace => {
            ui.enqueue(Intent::BuildPlace);
        }
        Action::Advanced => {
            ui.enqueue(Intent::Advanced);
        }
        Action::AddWaypoint => {
            ui.enqueue(Intent::AddWaypoint);
        }
        Action::Help(text) => {
            ui.help = Some(text);
            ui.dragging = None;
        }
        Action::Close => {
            ui.close_requested = true;
            ui.dragging = None;
            ui.capture = None;
        }
    }
}
fn owns(game: &Game<'_>) -> bool {
    game.native_mod_screen() == Some(rusty::ID) && game.native_on(rusty::ID)
}
pub(super) fn mouse(game: &Game<'_>, at: [f32; 2], button: i32, pressed: bool) -> bool {
    if !owns(game) {
        return false;
    }
    MENU.with_borrow_mut(|m| {
        if !m.ui.open {
            return false;
        }
        if !pressed {
            if m.ui.dragging.take().is_some() {
                m.ui.enqueue(Intent::Flush);
            }
            return true;
        }
        let at = at.map(|n| n / m.ui.render_scale);
        if !at.iter().all(|n| n.is_finite()) {
            return true;
        }
        // Clear remains an explicit command even while key capture is active.
        if m.ui.capture.is_some() && button == 0 {
            if let Some(hit) =
                m.ui.hits
                    .iter()
                    .rev()
                    .find(|h| inside(at, h.rect) && matches!(h.action, Action::ClearBind(..)))
                    .cloned()
            {
                act(m, hit.action, at, false);
                return true;
            }
        }
        if let Some(name) = m.ui.capture.take() {
            if (0..=31).contains(&button) {
                m.ui.enqueue(Intent::Bind(
                    name,
                    mouse_binding(button).expect("validated mouse button"),
                ));
            }
            return true;
        }
        if button != 0 && button != 1 {
            return true;
        }
        if let Some(hit) = m.ui.hits.iter().rev().find(|h| inside(at, h.rect)).cloned() {
            act(m, hit.action, at, button == 1);
        } else if m.ui.help.is_some() {
            m.ui.help = None;
        } else {
            m.ui.search_focus = false;
            m.ui.categories_open = false;
        }
        true
    })
}
pub(super) fn drag(game: &Game<'_>, at: [f32; 2]) -> bool {
    if !owns(game) {
        return false;
    }
    MENU.with_borrow_mut(|m| {
        let at = at.map(|n| n / m.ui.render_scale);
        if let Some((id, rect, kind)) = m.ui.dragging
            && let Some(value) = numeric_value(kind, f64::from((at[0] - rect[0]) / rect[2]))
        {
            m.ui.enqueue(Intent::Numeric(id, value));
        }
        m.ui.open
    })
}
pub(super) fn character(game: &Game<'_>, ch: char) -> bool {
    if !owns(game) {
        return false;
    }
    MENU.with_borrow_mut(|m| {
        m.ui.character(
            ch,
            game.modifiers.control_key(),
            game.modifiers.super_key(),
            game.modifiers.alt_key(),
        );
        m.ui.open
    })
}
pub(super) fn scroll(game: &Game<'_>, at: [f32; 2], amount: f32) -> bool {
    if !owns(game) {
        return false;
    }
    MENU.with_borrow_mut(|m| {
        let at = at.map(|n| n / m.ui.render_scale);
        if amount.is_finite() && m.ui.help.is_none() && !m.ui.categories_open {
            if inside(at, m.ui.list_rect) {
                m.ui.list_scroll = (m.ui.list_scroll - amount * 28.0)
                    .clamp(0.0, (m.ui.list_height - m.ui.list_rect[3]).max(0.0));
            } else if inside(at, m.ui.detail_rect) {
                m.ui.detail_scroll = (m.ui.detail_scroll - amount * 28.0)
                    .clamp(0.0, (m.ui.detail_height - m.ui.detail_rect[3]).max(0.0));
            }
        }
        m.ui.open
    })
}
pub(super) fn key(game: &Game<'_>, key: &str, pressed: bool, repeat: bool, modifiers: u32) -> bool {
    if !owns(game) {
        return false;
    }
    let menu_key = game.native_mod_key(rusty::ID, "menu", "key.keyboard.right.shift");
    MENU.with_borrow_mut(|m| {
        if !m.ui.open {
            return false;
        }
        if !pressed {
            return true;
        }
        let shift = modifiers & 1 != 0;
        let control = text_shortcut(modifiers & 2 != 0, modifiers & 8 != 0, modifiers & 4 != 0);
        if let Some(name) = m.ui.capture {
            if repeat {
                return true;
            }
            m.ui.capture = None;
            if key != "key.keyboard.escape" {
                m.ui.enqueue(Intent::Bind(name, key.into()));
            } else {
                m.ui.status = "Binding unchanged".into();
            }
            return true;
        }
        if key == "key.keyboard.escape" && !repeat {
            if m.ui.help.take().is_none() {
                if m.ui.categories_open {
                    m.ui.categories_open = false;
                } else {
                    m.ui.close_requested = true;
                }
            }
            return true;
        }
        if key == menu_key && !repeat {
            m.ui.close_requested = true;
            return true;
        }
        if m.ui.help.is_some() {
            if matches!(key, "key.keyboard.enter" | "key.keyboard.space") && !repeat {
                m.ui.help = None;
            }
            return true;
        }
        if control && key == "key.keyboard.f" {
            m.ui.search_focus = true;
            m.ui.query.move_to(0, false);
            m.ui.query.move_to(m.ui.query.value.chars().count(), true);
            return true;
        }
        if m.ui.search_focus {
            match key {
                "key.keyboard.a" if control => {
                    m.ui.query.move_to(0, false);
                    m.ui.query.move_to(m.ui.query.value.chars().count(), true);
                }
                "key.keyboard.c" | "key.keyboard.x" if control => {
                    let text = m.ui.query.selected();
                    m.ui.enqueue(Intent::Copy(text));
                    if key == "key.keyboard.x" {
                        m.ui.query.insert("");
                        m.ui.query_changed();
                    }
                }
                "key.keyboard.v" if control => {
                    m.ui.enqueue(Intent::Paste);
                }
                "key.keyboard.backspace" => {
                    m.ui.query.backspace();
                    m.ui.query_changed();
                }
                "key.keyboard.delete" => {
                    m.ui.query.delete();
                    m.ui.query_changed();
                }
                "key.keyboard.left" => {
                    m.ui.query
                        .move_to(m.ui.query.caret.saturating_sub(1), shift);
                }
                "key.keyboard.right" => {
                    m.ui.query.move_to(m.ui.query.caret + 1, shift);
                }
                "key.keyboard.home" => m.ui.query.move_to(0, shift),
                "key.keyboard.end" => m.ui.query.move_to(m.ui.query.value.chars().count(), shift),
                "key.keyboard.enter" | "key.keyboard.down" => {
                    let first = catalog::search_active(
                        &m.ui.query.value,
                        m.ui.category,
                        m.ui.collection,
                        &m.save.value.client,
                        enabled,
                    )
                    .first()
                    .map(|h| h.module.id);
                    if let Some(id) = first {
                        m.ui.selected = id;
                        m.ui.reveal_setting = true;
                        m.ui.search_focus = false;
                        m.ui.enqueue(Intent::Visit(id));
                    }
                }
                _ => {}
            }
        }
        if key == "key.keyboard.tab" && !repeat && !m.ui.hits.is_empty() {
            m.ui.search_focus = false;
            let n = m.ui.hits.len();
            m.ui.keyboard_hit = Some(match m.ui.keyboard_hit {
                None => {
                    if shift {
                        n - 1
                    } else {
                        0
                    }
                }
                Some(i) => {
                    if shift {
                        (i + n - 1) % n
                    } else {
                        (i + 1) % n
                    }
                }
            });
            return true;
        }
        if !m.ui.search_focus {
            if matches!(key, "key.keyboard.enter" | "key.keyboard.space") && !repeat {
                if let Some(hit) = m.ui.keyboard_hit.and_then(|i| m.ui.hits.get(i)).cloned() {
                    if matches!(hit.action, Action::Numeric(..)) {
                        // Keyboard sliders adjust with Left/Right, without
                        // jumping to a synthetic pointer position on Enter.
                        return true;
                    }
                    let at = [
                        hit.rect[0] + hit.rect[2] * 0.5,
                        hit.rect[1] + hit.rect[3] * 0.5,
                    ];
                    act(m, hit.action, at, false);
                }
            } else if matches!(key, "key.keyboard.left" | "key.keyboard.right") {
                if let Some(Action::Numeric(id, _, _)) =
                    m.ui.keyboard_hit
                        .and_then(|i| m.ui.hits.get(i))
                        .map(|h| h.action.clone())
                {
                    let steps = if key == "key.keyboard.left" {
                        -1.0
                    } else {
                        1.0
                    };
                    m.ui.enqueue(Intent::Nudge(id, steps));
                    m.ui.enqueue(Intent::Flush);
                }
            } else if matches!(
                key,
                "key.keyboard.up"
                    | "key.keyboard.down"
                    | "key.keyboard.page.up"
                    | "key.keyboard.page.down"
            ) {
                let delta = if key.contains("up") { -28.0 } else { 28.0 };
                m.ui.detail_scroll = (m.ui.detail_scroll + delta)
                    .clamp(0.0, (m.ui.detail_height - m.ui.detail_rect[3]).max(0.0));
            }
        }
        true
    })
}

#[derive(Clone, Copy)]
struct Colors {
    panel: u32,
    raised: u32,
    selected: u32,
    border: u32,
    accent: u32,
    text: u32,
    muted: u32,
}
fn fade_color(value: u32, opacity: f32) -> u32 {
    (((value >> 24) as f32 * opacity).round() as u32) << 24 | value & 0x00ffffff
}
impl Colors {
    fn faded(self, opacity: f32) -> Self {
        Self {
            panel: fade_color(self.panel, opacity),
            raised: fade_color(self.raised, opacity),
            selected: fade_color(self.selected, opacity),
            border: fade_color(self.border, opacity),
            accent: fade_color(self.accent, opacity),
            text: fade_color(self.text, opacity),
            muted: fade_color(self.muted, opacity),
        }
    }
}
fn colors(theme: Theme) -> Colors {
    match theme {
        Theme::Classic => Colors {
            panel: 0xf5232422,
            raised: 0xff343631,
            selected: 0xff4a4d44,
            border: 0xff777b6b,
            accent: 0xffdda878,
            text: 0xfff0f0e9,
            muted: 0xffb6baae,
        },
        Theme::Graphite => Colors {
            panel: 0xf51d1f20,
            raised: 0xff2c2f31,
            selected: 0xff41464a,
            border: 0xff666d70,
            accent: 0xffc4c9c9,
            text: 0xffeeeeeb,
            muted: 0xffa6adae,
        },
        Theme::Copper => Colors {
            panel: 0xf523211f,
            raised: 0xff36302a,
            selected: 0xff514036,
            border: 0xff846752,
            accent: 0xffdba576,
            text: 0xfff1e9df,
            muted: 0xffbaa99b,
        },
    }
}
fn fill(g: &Game<'_>, b: &mut HudBatch, r: [f32; 4], color: u32) {
    if r[2] > 0.0 && r[3] > 0.0 {
        g.native_fill(b, r, color);
    }
}
fn text(g: &Game<'_>, b: &mut HudBatch, s: &str, x: f32, y: f32, color: u32) {
    g.native_text(b, s, [x, y], color, false);
}
fn fit(g: &Game<'_>, s: &str, width: f32) -> String {
    if g.native_text_width(s) <= width {
        return s.into();
    }
    let mut out = String::new();
    for c in s.chars() {
        let next = format!("{out}{c}..");
        if g.native_text_width(&next) > width {
            break;
        }
        out.push(c);
    }
    out.push_str("..");
    out
}
fn wrapped(g: &Game<'_>, s: &str, width: f32, limit: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        let next = if line.is_empty() {
            word.into()
        } else {
            format!("{line} {word}")
        };
        if g.native_text_width(&next) > width && !line.is_empty() {
            lines.push(line);
            if lines.len() == limit {
                return lines;
            }
            line = word.into();
        } else {
            line = next;
        }
    }
    if !line.is_empty() && lines.len() < limit {
        lines.push(line);
    }
    lines
}
#[allow(clippy::too_many_arguments)]
fn button(
    g: &Game<'_>,
    b: &mut HudBatch,
    r: [f32; 4],
    label: &str,
    action: Action,
    chosen: bool,
    c: Colors,
    clip: Option<[f32; 4]>,
    m: &mut Model,
) {
    let Some(hit) = clip.map_or(Some(r), |v| intersect(r, v)) else {
        return;
    };
    if let Action::Numeric(id, rect, kind) = &action {
        if m.dragging.is_some_and(|(held, _, _)| held == *id) {
            m.dragging = Some((*id, *rect, *kind));
        }
    }
    let hover = inside(
        [g.cursor.0 / m.render_scale, g.cursor.1 / m.render_scale],
        hit,
    ) || m.keyboard_hit == Some(m.hits.len());
    // Native Minecraft nine-slice sprites; one transform serves rendering,
    // mouse/controller targets, automation labels and narration rectangles.
    let sprite = if hover || chosen {
        "gui/sprites/widget/button_highlighted"
    } else {
        "gui/sprites/widget/button"
    };
    horizon_client::ui::nine_slice(b, &g.gui.atlas, sprite, r[0], r[1], r[2], r[3], u32::MAX);
    let fitted = fit(g, label, r[2] - 10.0);
    let label_width = g.native_text_width(&fitted);
    let label_y = if matches!(&action, Action::Select(..)) {
        r[1] + MODULE_TITLE_Y
    } else {
        r[1] + ((r[3] - 8.0) / 2.0).floor()
    };
    let label_x = if matches!(&action, Action::Select(..)) {
        r[0] + 6.0
    } else {
        r[0] + (r[2] - label_width) * 0.5
    };
    g.native_text(b, &fitted, [label_x, label_y], c.text, true);
    if m.record_controls {
        let physical = hit.map(|v| v * m.render_scale);
        horizon_client::widgets::record(label, physical, true);
        horizon_client::widgets::narratable(|| {
            horizon_client::widgets::Narratable::button(label, physical, true).hovered(hover)
        });
        horizon_client::widgets::widget_cursor(hover, true);
    }
    if chosen {
        fill(
            g,
            b,
            [r[0] + 1.0, r[1] + r[3] - 2.0, r[2] - 2.0, 1.0],
            c.accent,
        );
    }
    if m.hits.len() < 256 {
        m.hits.push(Hit { rect: hit, action });
        if m.record_controls {
            horizon_client::widgets::target(hit.map(|v| v * m.render_scale));
        }
    }
}
fn scrollbar(g: &Game<'_>, b: &mut HudBatch, r: [f32; 4], content: f32, offset: f32, c: Colors) {
    if content > r[3] && r[3] > 0.0 {
        let h = (r[3] * r[3] / content).max(8.0).min(r[3]);
        let y = r[1] + offset / (content - r[3]) * (r[3] - h);
        fill(g, b, [r[0] + r[2] - 2.0, y, 2.0, h], c.border);
    }
}
fn key_label(s: &str) -> String {
    match s {
        "key.keyboard.right.shift" => "Right Shift".into(),
        "key.keyboard.left.shift" => "Left Shift".into(),
        "key.keyboard.unknown" => "Unbound".into(),
        _ => s
            .strip_prefix("key.keyboard.")
            .map(|v| v.replace('.', " ").to_uppercase())
            .or_else(|| s.strip_prefix("key.mouse.").map(|v| format!("Mouse {v}")))
            .unwrap_or_else(|| s.into()),
    }
}
fn numeric_bounds(kind: SettingKind) -> Option<(f64, f64, f64)> {
    match kind {
        SettingKind::Integer { min, max, step, .. } => Some((min as f64, max as f64, step as f64)),
        SettingKind::Decimal { min, max, step, .. } => Some((min, max, step)),
        _ => None,
    }
}

pub(super) fn draw(game: &Game<'_>, batch: &mut HudBatch) {
    if !owns(game) {
        return;
    }
    MENU.with_borrow_mut(|menu| {
        let transform = MenuTransform::new(
            [batch.width, batch.height],
            menu.save.value.client.menu_scale,
        );
        menu.ui.render_scale = transform.scale;
        let viewport = [batch.width, batch.height];
        batch.width = transform.logical_size[0];
        batch.height = transform.logical_size[1];
        let start = batch.vertices.len();
        draw_menu(game, batch, menu);
        batch.width = viewport[0];
        batch.height = viewport[1];
        for vertex in &mut batch.vertices[start..] {
            vertex.pos = transform.point(vertex.pos);
        }
    });
}

fn draw_menu(game: &Game<'_>, batch: &mut HudBatch, menu: &mut Menu) {
    if !menu.ui.open {
        return;
    }

    let cfg = menu.save.value.clone();
    let m = &mut menu.ui;
    let fade = opening_opacity(
        menu.opened_at.elapsed().as_secs_f32() * 1000.0,
        cfg.client.reduced_motion,
    );
    let c = colors(cfg.client.theme).faded(fade);
    m.hits.clear();
    m.record_controls = !m.categories_open && m.help.is_none();

    let w = (batch.width - 12.0).min(750.0).max(1.0);
    let h = (batch.height - 12.0).min(460.0).max(1.0);
    let x = (batch.width - w) * 0.5;
    let y = (batch.height - h) * 0.5;

    fill(
        game,
        batch,
        [0.0, 0.0, batch.width, batch.height],
        fade_color(0x80101010, fade),
    );
    fill(game, batch, [x - 1.0, y - 1.0, w + 2.0, h + 2.0], c.border);
    fill(game, batch, [x, y, w, h], c.panel);

    if w < 230.0 || h < 160.0 {
        text(
            game,
            batch,
            "Rusty: increase window or lower GUI scale",
            x + 5.0,
            y + 8.0,
            c.text,
        );
        button(
            game,
            batch,
            [x + 5.0, y + h - 25.0, (w - 10.0).max(20.0), 20.0],
            "Close",
            Action::Close,
            false,
            c,
            None,
            m,
        );
        return;
    }

    fill(game, batch, [x, y, w, 2.0], c.accent);
    game.native_sprite(
        batch,
        horizon_client::hud::MOD_IMAGE_SPRITE,
        [0.0, 0.0, 64.0, 64.0],
        [x + 8.0, y + 4.0, 24.0, 24.0],
    );
    text(
        game,
        batch,
        &format!("{} MODULES", catalog::MODULES.len()),
        x + 41.0,
        y + 12.0,
        c.muted,
    );

    button(game, batch, [x+w-49.0,y+5.0,20.0,21.0], "?", Action::Help("Search names, aliases, descriptions and setting labels. All words must match. Click a module to see controls; ON/OFF toggles it and * favorites it. Drag sliders or use -/+; right-click a slider to reset. Tab and Enter navigate; Ctrl+F focuses search. Escape closes help or the menu. World ticks continue while this screen is open.".into()), false, c, None, m);

    button(
        game,
        batch,
        [x + w - 25.0, y + 5.0, 20.0, 21.0],
        "X",
        Action::Close,
        false,
        c,
        None,
        m,
    );

    let sr = [x + 8.0, y + 33.0, w - 16.0, 22.0];
    fill(game, batch, sr, 0xff0b0b0b);
    fill(
        game,
        batch,
        [sr[0] + 1.0, sr[1] + 1.0, sr[2] - 2.0, sr[3] - 2.0],
        0xff191613,
    );
    fill(
        game,
        batch,
        [sr[0], sr[1] + sr[3] - 1.0, sr[2], 1.0],
        c.border,
    );
    fill(
        game,
        batch,
        [sr[0] + sr[2] - 1.0, sr[1], 1.0, sr[3]],
        c.border,
    );
    fill(
        game,
        batch,
        [sr[0], sr[1] + 21.0, sr[2], 1.0],
        if m.search_focus { c.accent } else { c.border },
    );

    let query_chars: Vec<char> = m.query.value.chars().collect();
    let mut display_start = 0;
    while display_start < m.query.caret {
        let before: String = query_chars[display_start..m.query.caret].iter().collect();
        if game.native_text_width(&before) <= sr[2] - 15.0 {
            break;
        }
        display_start += 1;
    }
    let visible_query: String = query_chars[display_start..].iter().collect();
    let shown = if m.query.value.is_empty() {
        "Search modules or settings...".into()
    } else {
        fit(game, &visible_query, sr[2] - 12.0)
    };

    text(
        game,
        batch,
        &shown,
        sr[0] + 6.0,
        sr[1] + 7.0,
        if m.query.value.is_empty() {
            c.muted
        } else {
            c.text
        },
    );

    if m.search_focus {
        let before: String = query_chars[display_start..m.query.caret].iter().collect();
        let cx = sr[0] + 6.0 + game.native_text_width(&before).min(sr[2] - 13.0);
        fill(game, batch, [cx, sr[1] + 5.0, 1.0, 12.0], c.accent);
        if m.query.caret != m.query.anchor {
            fill(
                game,
                batch,
                [sr[0] + 4.0, sr[1] + 19.0, sr[2] - 8.0, 1.0],
                c.accent,
            );
        }
    }

    m.hits.push(Hit {
        rect: sr,
        action: Action::Search,
    });
    if m.record_controls {
        horizon_client::widgets::target(sr.map(|v| v * m.render_scale));
    }

    let tabw = (w - 22.0) / 4.0;
    for (i, (label, value)) in [
        ("All", Collection::All),
        ("Enabled", Collection::Enabled),
        ("Favorites", Collection::Favorites),
        ("Recent", Collection::Recent),
    ]
    .into_iter()
    .enumerate()
    {
        button(
            game,
            batch,
            [x + 8.0 + i as f32 * (tabw + 2.0), y + 60.0, tabw, 20.0],
            label,
            Action::Collection(value),
            m.collection == value,
            c,
            None,
            m,
        );
    }

    let filterw = (w - 20.0) * 0.43;
    let themew = (w - 20.0) * 0.30;
    let motionw = (w - 20.0) - filterw - themew;

    let count = catalog::MODULES
        .iter()
        .filter(|s| m.category.is_none_or(|cat| s.category == cat))
        .count();

    let category = m.category.map_or("All categories", |cat| cat.label());

    button(
        game,
        batch,
        [x + 8.0, y + 85.0, filterw, 20.0],
        &format!("{category} ({count}) v"),
        Action::CategoryPicker,
        m.categories_open,
        c,
        None,
        m,
    );

    button(
        game,
        batch,
        [x + 10.0 + filterw, y + 85.0, themew, 20.0],
        match cfg.client.theme {
            Theme::Classic => "Classic",
            Theme::Graphite => "Graphite",
            Theme::Copper => "Copper",
        },
        Action::Theme,
        false,
        c,
        None,
        m,
    );

    button(
        game,
        batch,
        [x + 12.0 + filterw + themew, y + 85.0, motionw, 20.0],
        if cfg.client.reduced_motion {
            "Motion: Low"
        } else {
            "Motion: Full"
        },
        Action::Motion,
        false,
        c,
        None,
        m,
    );

    let lw = ((w - 25.0) * 0.40).clamp(92.0, 240.0);
    let content_h = (h - 178.0).max(12.0);
    m.list_rect = [x + 8.0, y + 112.0, lw, content_h];
    m.detail_rect = [x + lw + 17.0, y + 112.0, w - lw - 25.0, content_h];

    fill(
        game,
        batch,
        [x + lw + 12.0, y + 112.0, 1.0, content_h],
        c.border,
    );

    let results = catalog::search_active(
        &m.query.value,
        m.category,
        m.collection,
        &cfg.client,
        |id| enabled(id),
    );

    if !results.iter().any(|hit| hit.module.id == m.selected)
        && let Some(first) = results.first()
    {
        m.selected = first.module.id;
        m.detail_scroll = 0.0;
    }

    m.list_height = results.len() as f32 * MODULE_ROW_PITCH;
    m.list_scroll = m
        .list_scroll
        .clamp(0.0, (m.list_height - content_h).max(0.0));

    let start = batch.vertices.len();
    let target_start = horizon_client::widgets::targets_mark();

    for (i, hit) in results.iter().enumerate() {
        let module = hit.module;
        let yy = m.list_rect[1] + i as f32 * MODULE_ROW_PITCH - m.list_scroll;
        let r = [m.list_rect[0], yy, lw - 4.0, MODULE_ROW_HEIGHT];
        let chosen = module.id == m.selected;

        button(
            game,
            batch,
            [r[0], r[1], r[2] - 49.0, r[3]],
            module.name,
            Action::Select(module.id),
            chosen,
            c,
            Some(m.list_rect),
            m,
        );

        // A separate category baseline leaves a full seven pixels below the text.
        text(
            game,
            batch,
            &fit(
                game,
                hit.setting.map_or(module.category.label(), |s| s.label),
                r[2] - 57.0,
            ),
            r[0] + 4.0,
            r[1] + MODULE_CATEGORY_Y,
            c.muted,
        );

        button(
            game,
            batch,
            [r[0] + r[2] - 47.0, yy, 28.0, MODULE_ROW_HEIGHT],
            if enabled(module.id) { "ON" } else { "OFF" },
            Action::Toggle(module.id),
            enabled(module.id),
            c,
            Some(m.list_rect),
            m,
        );

        button(
            game,
            batch,
            [r[0] + r[2] - 17.0, yy, 17.0, MODULE_ROW_HEIGHT],
            if cfg.client.is_favorite(module.id) {
                "*"
            } else {
                "+"
            },
            Action::Favorite(module.id),
            cfg.client.is_favorite(module.id),
            c,
            Some(m.list_rect),
            m,
        );
    }

    if results.is_empty() {
        text(
            game,
            batch,
            "No matching modules",
            m.list_rect[0] + 2.0,
            m.list_rect[1] + 6.0,
            c.muted,
        );
    }

    batch.clip_since(
        start,
        m.list_rect[0],
        m.list_rect[1],
        m.list_rect[0] + lw,
        m.list_rect[1] + content_h,
    );
    horizon_client::widgets::clip_targets(
        target_start,
        m.list_rect[1] * m.render_scale,
        (m.list_rect[1] + content_h) * m.render_scale,
    );

    scrollbar(game, batch, m.list_rect, m.list_height, m.list_scroll, c);

    if let Some(module) = results
        .iter()
        .find(|hit| hit.module.id == m.selected)
        .map(|hit| hit.module)
    {
        let found = results
            .iter()
            .find(|hit| hit.module.id == m.selected)
            .and_then(|hit| hit.setting)
            .map(|s| s.id);

        if m.reveal_setting {
            if let Some(id) = found {
                let mut offset = 98.0;
                for s in module.settings {
                    if s.id == id {
                        break;
                    }
                    offset += if numeric_bounds(s.kind).is_some() {
                        46.0
                    } else {
                        27.0
                    };
                }
                m.detail_scroll = offset;
            }
            m.reveal_setting = false;
        }

        let hud_controls = hud_layout::CONTROLS.iter().find(|(id, _)| *id == module.id);
        let total = 98.0
            + if hud_controls.is_some() { 185.0 } else { 0.0 }
            + module
                .settings
                .iter()
                .map(|s| {
                    if numeric_bounds(s.kind).is_some() {
                        46.0
                    } else {
                        27.0
                    }
                })
                .sum::<f32>();
        m.detail_height = total;
        m.detail_scroll = m.detail_scroll.clamp(0.0, (total - content_h).max(0.0));

        let r = m.detail_rect;
        let sx = r[0] + 3.0;
        let sw = r[2] - 8.0;
        let mut yy = r[1] - m.detail_scroll;
        let start = batch.vertices.len();
        let target_start = horizon_client::widgets::targets_mark();

        text(
            game,
            batch,
            &fit(game, module.name, sw - 20.0),
            sx,
            yy + 3.0,
            c.text,
        );
        button(
            game,
            batch,
            [sx + sw - 17.0, yy, 17.0, 18.0],
            "?",
            Action::Help(module.description.into()),
            false,
            c,
            Some(r),
            m,
        );
        text(game, batch, module.category.label(), sx, yy + 19.0, c.muted);
        yy += 38.0;

        button(
            game,
            batch,
            [sx, yy, sw, 21.0],
            if module.id == "zoom" {
                "Hold key to zoom"
            } else if enabled(module.id) {
                "Enabled - click to disable"
            } else {
                "Disabled - click to enable"
            },
            Action::Toggle(module.id),
            enabled(module.id),
            c,
            Some(r),
            m,
        );
        yy += 27.0;

        if let Some(name) = module.keybind_id {
            let current = game.native_mod_key(
                rusty::ID,
                name,
                module.default_key.unwrap_or("key.keyboard.unknown"),
            );
            button(
                game,
                batch,
                [sx, yy, (sw - 47.0).max(20.0), 21.0],
                &format!("Bind: {}", key_label(&current)),
                Action::Bind(name),
                m.capture == Some(name),
                c,
                Some(r),
                m,
            );
            button(
                game,
                batch,
                [sx + sw - 44.0, yy, 44.0, 21.0],
                "Clear",
                Action::ClearBind(name),
                false,
                c,
                Some(r),
                m,
            );
        } else {
            text(
                game,
                batch,
                "No dedicated hotkey",
                sx + 2.0,
                yy + 6.0,
                c.muted,
            );
        }
        yy += 33.0;

        for s in module.settings {
            let selected = found == Some(s.id);
            if selected {
                fill(
                    game,
                    batch,
                    [
                        sx - 1.0,
                        yy - 2.0,
                        sw + 2.0,
                        if numeric_bounds(s.kind).is_some() {
                            43.0
                        } else {
                            24.0
                        },
                    ],
                    c.selected,
                );
            }

            match s.kind {
                SettingKind::Bool { .. } => {
                    let value = flag(&cfg, s.id).unwrap_or(false);
                    button(
                        game,
                        batch,
                        [sx, yy, sw - 20.0, 21.0],
                        &format!("{}: {}", s.label, if value { "ON" } else { "OFF" }),
                        Action::Bool(s.id),
                        value,
                        c,
                        Some(r),
                        m,
                    );
                    button(
                        game,
                        batch,
                        [sx + sw - 17.0, yy, 17.0, 21.0],
                        "?",
                        Action::Help(s.description.into()),
                        false,
                        c,
                        Some(r),
                        m,
                    );
                    yy += 27.0;
                }
                SettingKind::Choice { .. } => {
                    button(
                        game,
                        batch,
                        [sx, yy, sw - 20.0, 21.0],
                        &format!(
                            "{}: {}",
                            s.label,
                            match cfg.activation {
                                ActivationMode::Toggle => "Toggle",
                                ActivationMode::Hold => "Hold",
                            }
                        ),
                        Action::Choice,
                        false,
                        c,
                        Some(r),
                        m,
                    );
                    button(
                        game,
                        batch,
                        [sx + sw - 17.0, yy, 17.0, 21.0],
                        "?",
                        Action::Help(s.description.into()),
                        false,
                        c,
                        Some(r),
                        m,
                    );
                    yy += 27.0;
                }
                SettingKind::Action => {
                    button(
                        game,
                        batch,
                        [sx, yy, sw, 21.0],
                        s.label,
                        match s.id {
                            "add_waypoint" => Action::AddWaypoint,
                            "build_anchor" => Action::BuildAnchor,
                            "build_place" => Action::BuildPlace,
                            _ => Action::Advanced,
                        },
                        false,
                        c,
                        Some(r),
                        m,
                    );
                    yy += 27.0;
                }
                kind => {
                    let (min, max, _) = numeric_bounds(kind).unwrap();
                    let value = number(&cfg, s.id);
                    text(
                        game,
                        batch,
                        &fit(game, s.label, sw - 18.0),
                        sx + 2.0,
                        yy + 2.0,
                        c.text,
                    );
                    button(
                        game,
                        batch,
                        [sx + sw - 16.0, yy - 2.0, 16.0, 16.0],
                        "?",
                        Action::Help(s.description.into()),
                        false,
                        c,
                        Some(r),
                        m,
                    );

                    let track = [sx + 21.0, yy + 17.0, (sw - 44.0).max(12.0), 19.0];
                    button(
                        game,
                        batch,
                        [sx, yy + 17.0, 18.0, 19.0],
                        "-",
                        Action::Step(s.id, -1.0),
                        false,
                        c,
                        Some(r),
                        m,
                    );
                    button(
                        game,
                        batch,
                        [sx + sw - 18.0, yy + 17.0, 18.0, 19.0],
                        "+",
                        Action::Step(s.id, 1.0),
                        false,
                        c,
                        Some(r),
                        m,
                    );

                    button(
                        game,
                        batch,
                        track,
                        &format!("{value:.2}"),
                        Action::Numeric(s.id, track, kind),
                        m.dragging.is_some_and(|(id, _, _)| id == s.id),
                        c,
                        Some(r),
                        m,
                    );
                    let t = ((value - min) / (max - min)).clamp(0.0, 1.0) as f32;
                    fill(
                        game,
                        batch,
                        [
                            track[0] + t * (track[2] - 3.0),
                            track[1] + track[3] - 3.0,
                            3.0,
                            3.0,
                        ],
                        c.accent,
                    );
                    yy += 46.0;
                }
            }
        }

        if let Some((panel, ids)) = hud_controls {
            text(
                game,
                batch,
                "Screen HUD position and size",
                sx,
                yy + 3.0,
                c.accent,
            );
            yy += 20.0;
            for (axis, id) in ids.iter().enumerate() {
                local_slider(
                    game,
                    batch,
                    m,
                    &cfg,
                    *id,
                    ["HUD X (%)", "HUD Y (%)", "HUD size"][axis],
                    [sx, yy, sw, 46.0],
                    r,
                    c,
                );
                yy += 46.0;
            }
            button(
                game,
                batch,
                [sx, yy, sw, 21.0],
                "Reset HUD position / size",
                Action::ResetHud(panel),
                false,
                c,
                Some(r),
                m,
            );
        }
        batch.clip_since(start, r[0], r[1], r[0] + r[2], r[1] + r[3]);
        horizon_client::widgets::clip_targets(
            target_start,
            r[1] * m.render_scale,
            (r[1] + r[3]) * m.render_scale,
        );
        scrollbar(game, batch, r, total, m.detail_scroll, c);
    } else {
        m.detail_height = 0.0;
        text(
            game,
            batch,
            "Change search or collection",
            m.detail_rect[0] + 3.0,
            m.detail_rect[1] + 6.0,
            c.muted,
        );
    }

    let scale_y = y + h - 63.0;
    button(
        game,
        batch,
        [x + 8.0, scale_y, 22.0, 21.0],
        "-",
        Action::Step("menu_scale", -1.0),
        false,
        c,
        None,
        m,
    );
    button(
        game,
        batch,
        [x + 33.0, scale_y, w - 103.0, 21.0],
        &format!(
            "Menu size: {:.0}% (right-click resets)",
            catalog::menu_percent(cfg.client.menu_scale)
        ),
        Action::Numeric(
            "menu_scale",
            [x + 33.0, scale_y, w - 103.0, 21.0],
            setting_kind("menu_scale").unwrap(),
        ),
        false,
        c,
        None,
        m,
    );
    button(
        game,
        batch,
        [x + w - 67.0, scale_y, 22.0, 21.0],
        "+",
        Action::Step("menu_scale", 1.0),
        false,
        c,
        None,
        m,
    );
    button(
        game,
        batch,
        [x + w - 42.0, scale_y, 34.0, 21.0],
        "100%",
        Action::ResetMenuScale,
        false,
        c,
        None,
        m,
    );

    text(
        game,
        batch,
        &fit(game, &m.status, w - 16.0),
        x + 8.0,
        y + h - 32.0,
        if m.error { 0xffffad89 } else { c.muted },
    );

    let menukey = game.native_mod_key(rusty::ID, "menu", "key.keyboard.right.shift");
    button(
        game,
        batch,
        [x + 8.0, y + h - 22.0, (w - 20.0) * 0.55 - 47.0, 18.0],
        &format!("Menu: {}", key_label(&menukey)),
        Action::Bind("menu"),
        m.capture == Some("menu"),
        c,
        None,
        m,
    );

    button(
        game,
        batch,
        [x + 8.0 + (w - 20.0) * 0.55 - 44.0, y + h - 22.0, 44.0, 18.0],
        "Clear",
        Action::ClearBind("menu"),
        false,
        c,
        None,
        m,
    );

    button(
        game,
        batch,
        [
            x + 10.0 + (w - 20.0) * 0.55,
            y + h - 22.0,
            (w - 20.0) * 0.45,
            18.0,
        ],
        "Advanced / Profiles",
        Action::Advanced,
        false,
        c,
        None,
        m,
    );

    if m.categories_open {
        m.record_controls = true;
        let popup = [x + 8.0, y + 31.0, w - 16.0, h - 57.0];

        fill(game, batch, popup, c.panel);
        m.hits.clear();

        let options: Vec<_> = std::iter::once(None)
            .chain(
                Category::ALL
                    .iter()
                    .copied()
                    .filter(|cat| catalog::MODULES.iter().any(|s| s.category == *cat))
                    .map(Some),
            )
            .collect();

        let rows = ((popup[3] - 4.0) / 21.0).floor().max(1.0) as usize;

        let columns = options.len().div_ceil(rows);
        let cell = (popup[2] - 4.0) / columns as f32;

        let start = batch.vertices.len();

        for (i, cat) in options.into_iter().enumerate() {
            let label = cat.map_or("All categories", |c| c.label());
            let n = catalog::MODULES
                .iter()
                .filter(|s| cat.is_none_or(|c| s.category == c))
                .count();

            button(
                game,
                batch,
                [
                    popup[0] + 2.0 + (i / rows) as f32 * cell,
                    popup[1] + 2.0 + (i % rows) as f32 * 21.0,
                    cell - 2.0,
                    19.0,
                ],
                &format!("{label} ({n})"),
                Action::Category(cat),
                m.category == cat,
                c,
                Some(popup),
                m,
            );
        }

        batch.clip_since(
            start,
            popup[0],
            popup[1],
            popup[0] + popup[2],
            popup[1] + popup[3],
        );
    }

    if let Some(message) = m.help.clone() {
        let pw = (w - 22.0).min(420.0);
        let lines = wrapped(
            game,
            &message,
            pw - 18.0,
            ((h - 65.0) / 12.0).max(1.0) as usize,
        );
        let ph = (lines.len() as f32 * 12.0 + 35.0).min(h - 12.0);
        let px = x + (w - pw) * 0.5;
        let py = y + (h - ph) * 0.5;

        fill(game, batch, [x, y, w, h], 0xd0101010);
        fill(
            game,
            batch,
            [px - 1.0, py - 1.0, pw + 2.0, ph + 2.0],
            c.border,
        );
        fill(game, batch, [px, py, pw, ph], c.panel);
        text(
            game,
            batch,
            "HELP - Escape to dismiss",
            px + 8.0,
            py + 8.0,
            c.accent,
        );
        for (i, line) in lines.iter().enumerate() {
            text(
                game,
                batch,
                line,
                px + 8.0,
                py + 25.0 + i as f32 * 12.0,
                c.text,
            );
        }
        m.hits.clear();
    }

    if let Some(name) = m.capture {
        let label = format!("Binding {name}: press a key / mouse button, Escape cancels");
        fill(game, batch, [x, y + h - 39.0, w, 17.0], c.selected);
        text(
            game,
            batch,
            &fit(game, &label, w - 14.0),
            x + 7.0,
            y + h - 35.0,
            c.text,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn local_slider(
    g: &Game<'_>,
    b: &mut HudBatch,
    m: &mut Model,
    cfg: &Config,
    id: &'static str,
    label: &str,
    row: [f32; 4],
    clip: [f32; 4],
    c: Colors,
) {
    let [x, y, w, _] = row;
    let kind = setting_kind(id).expect("registered local numeric control");
    let value = number(cfg, id);
    text(g, b, label, x + 2.0, y + 2.0, c.text);
    let track = [x + 21.0, y + 17.0, (w - 44.0).max(12.0), 19.0];
    button(
        g,
        b,
        [x, y + 17.0, 18.0, 19.0],
        "-",
        Action::Step(id, -1.0),
        false,
        c,
        Some(clip),
        m,
    );
    button(
        g,
        b,
        track,
        &format!("{value:.2}"),
        Action::Numeric(id, track, kind),
        m.dragging.is_some_and(|(v, _, _)| v == id),
        c,
        Some(clip),
        m,
    );
    button(
        g,
        b,
        [x + w - 18.0, y + 17.0, 18.0, 19.0],
        "+",
        Action::Step(id, 1.0),
        false,
        c,
        Some(clip),
        m,
    );
}
