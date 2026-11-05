// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Received observations only. Never opens a container, changes light or moves the player.
use super::Game;
use horizon_client::{gizmos::Gizmos, hud::HudBatch};
use horizon_client_render::mesher::{rusty, rusty_awareness as logic};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Observations {
    refreshed: Option<Instant>,
    dimension: String,
    revision: u64,
    center: [i32; 3],
    geometry: Gizmos,
    light_cells: usize,
    storage_boxes: usize,
    storage_partial: bool,
    speed: Option<f64>,
    last_position: Option<([f64; 3], Instant)>,
    clicks: (u32, u32),
    entity_tracer_lines: usize,
    prediction_key: Option<([u64; 8], String)>,
    prediction_lines: Gizmos,
    prediction_count: usize,
    prediction_kind: &'static str,
    prediction_partial: bool,
    prediction_status: &'static str,
}
thread_local! { static OBS:RefCell<Observations>=RefCell::new(Observations::default()); }
fn clear_prediction(o: &mut Observations) {
    o.prediction_key = None;
    o.prediction_lines = Gizmos::default();
    o.prediction_count = 0;
    o.prediction_kind = "";
    o.prediction_status = "disabled";
    o.prediction_partial = false;
}
pub(super) fn reset() {
    OBS.with_borrow_mut(|o| *o = Observations::default());
}
fn loaded(game: &Game<'_>, pos: [i32; 3]) -> bool {
    pos[1] >= game.world.bottom_y()
        && pos[1] < game.world.top_y()
        && game.world.chunks.contains_key(&(pos[0] >> 4, pos[2] >> 4))
}
fn distance2(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum()
}
fn storage_color(name: &str) -> Option<u32> {
    Some(match name {
        "minecraft:trapped_chest" => 0xffff6d67,
        "minecraft:chest" | "minecraft:barrel" => 0xffe6b878,
        "minecraft:ender_chest" => 0xffbb88ed,
        "minecraft:hopper" | "minecraft:dispenser" | "minecraft:dropper" => 0xffa9b6c4,
        "minecraft:furnace" | "minecraft:blast_furnace" | "minecraft:smoker" => 0xffffa167,
        "minecraft:brewing_stand" => 0xffe2a0db,
        n if n.ends_with("shulker_box") => 0xffb894df,
        _ => return None,
    })
}
pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_window() || !game.native_on(rusty::ID) || game.native_player().is_none() {
        reset();
        return;
    }
    let cfg = rusty::config();
    let settings = &cfg.awareness;
    if !settings.trajectories {
        OBS.with_borrow_mut(clear_prediction);
    }
    if settings.clicks_hud {
        let clicks = game.native_clicks();
        OBS.with_borrow_mut(|o| o.clicks = clicks);
    }
    let player = game.native_player().unwrap();
    let at = [player.x, player.y, player.z];
    let center = at.map(|v| v.floor() as i32);
    OBS.with_borrow_mut(|o| {
        if o.dimension != player.dimension || o.revision != rusty::revision() {
            *o = Observations {
                dimension: player.dimension.into(),
                revision: rusty::revision(),
                ..Default::default()
            };
        }
        if !settings.light_overlay && !settings.storage_esp && !settings.speed_hud {
            o.geometry = Gizmos::default();
            o.last_position = None;
            o.speed = None;
            o.light_cells = 0;
            o.storage_boxes = 0;
            o.storage_partial = false;
            return;
        }
        if o.refreshed
            .is_some_and(|t| t.elapsed() < Duration::from_millis(250))
        {
            return;
        }
        let now = Instant::now();
        if let Some((old, time)) = o.last_position {
            let seconds = now.duration_since(time).as_secs_f64();
            let distance = (at[0] - old[0]).hypot(at[2] - old[2]);
            o.speed =
                (seconds > 0. && distance < 16. && seconds < 2.).then_some(distance / seconds);
        }
        o.last_position = Some((at, now));
        o.center = center;
        o.refreshed = Some(now);
        o.geometry = Gizmos::default();
        o.light_cells = 0;
        o.storage_boxes = 0;
        o.storage_partial = false;
        let radius = settings.overlay_radius as i32;
        let facts = game.res.facts();
        if settings.light_overlay {
            // Conservative full-block floors only; light risk is not a claim that a mob can spawn.
            for z in center[2] - radius..=center[2] + radius {
                for x in center[0] - radius..=center[0] + radius {
                    for y in center[1] - 2..=center[1] + 2 {
                        let pos = [x, y, z];
                        if !loaded(game, pos)
                            || !loaded(game, [x, y - 1, z])
                            || !loaded(game, [x, y + 1, z])
                        {
                            continue;
                        }
                        let chunk = &game.world.chunks[&(x >> 4, z >> 4)];
                        if !(chunk.lit || chunk.light_loaded)
                            || game.world.dirty_light.contains(&(x >> 4, z >> 4))
                        {
                            continue;
                        }
                        if !facts.get(game.world.block(x, y - 1, z)).full_collision
                            || !facts.is_air(game.world.block(x, y, z))
                            || !facts.is_air(game.world.block(x, y + 1, z))
                        {
                            continue;
                        }
                        let light = chunk.light(x & 15, y, z & 15);
                        if light & 15 > 0 {
                            continue;
                        }
                        let color = if light >> 4 == 0 {
                            0xffed6868
                        } else {
                            0xffe7c16c
                        };
                        o.geometry.line(
                            [f64::from(x) + 0.2, f64::from(y) + 0.025, f64::from(z) + 0.2],
                            [f64::from(x) + 0.8, f64::from(y) + 0.025, f64::from(z) + 0.8],
                            color,
                            1.5,
                        );
                        o.geometry.line(
                            [f64::from(x) + 0.8, f64::from(y) + 0.025, f64::from(z) + 0.2],
                            [f64::from(x) + 0.2, f64::from(y) + 0.025, f64::from(z) + 0.8],
                            color,
                            1.5,
                        );
                        o.light_cells += 1;
                    }
                }
            }
        }
        if settings.storage_esp {
            let mut storage = Vec::new();
            let mut examined = 0usize;
            // Chunk payload block entities are drained into this received live table.
            // The table is bounded per refresh; a dense world is marked partial.
            for (&(bx, by, bz), _) in game.world.block_entities.iter().take(4097) {
                examined += 1;
                if examined > 4096 {
                    o.storage_partial = true;
                    break;
                }
                let pos = [bx, by, bz];
                let distance = distance2(at, pos.map(|v| f64::from(v) + 0.5));
                if distance > f64::from(radius).powi(2) || !loaded(game, pos) {
                    continue;
                }
                if let Some(color) = storage_color(&facts.get(game.world.block(bx, by, bz)).name) {
                    storage.push((distance, pos, color));
                }
            }
            storage.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            o.storage_partial |= storage.len() > 128;
            for (_, pos, color) in storage.into_iter().take(128) {
                let at = pos.map(f64::from);
                let mut box_ = Gizmos::default();
                box_.cuboid(at, at.map(|v| v + 1.), color, 1.5);
                o.geometry.on_top.extend(box_.lines);
                o.storage_boxes += 1;
            }
        }
    });
}
fn trajectory(game: &Game<'_>, out: &mut Gizmos) {
    if !game.native_playing() || horizon_client_render::mesher::rusty_freecam::active() {
        OBS.with_borrow_mut(clear_prediction);
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };
    let settings = &rusty::config().awareness;
    let item = game
        .inventory
        .get(game.selected)
        .and_then(Option::as_ref)
        .map(|s| &*s.item)
        .unwrap_or("");
    let key = [
        game.world.game_time as u64,
        player.x.to_bits(),
        player.y.to_bits(),
        player.z.to_bits(),
        u64::from(player.yaw.to_bits()),
        u64::from(player.pitch.to_bits()),
        game.selected as u64,
        u64::from(settings.trajectory_steps),
    ];
    OBS.with_borrow_mut(|cache| {
        let same = cache
            .prediction_key
            .as_ref()
            .is_some_and(|(old, id)| *old == key && id == item);
        if !same {
            let prediction =
                super::creatures::rusty_prediction::predict(game, settings.trajectory_steps);
            cache.prediction_lines = Gizmos::default();
            for [from, to] in &prediction.segments {
                cache.prediction_lines.line(*from, *to, 0xff9dd6ac, 1.5);
            }
            if let Some(hit) = prediction.hit {
                cache.prediction_lines.cuboid(
                    hit.map(|v| v - 0.1),
                    hit.map(|v| v + 0.1),
                    0xffe7c16c,
                    2.,
                );
            }
            cache.prediction_count = prediction.segments.len();
            cache.prediction_kind = prediction.kind;
            cache.prediction_status = prediction.status;
            cache.prediction_partial = prediction.partial;
            cache.prediction_key = Some((key, item.into()));
        }
        out.lines.extend_from_slice(&cache.prediction_lines.lines);
    });
}
pub(super) fn gizmos(game: &Game<'_>, out: &mut Gizmos, camera: [f64; 3], view: glam::Mat4) {
    if !game.native_window()
        || !game.native_on(rusty::ID)
        || game.ui.hide_gui
        || game.native_player().is_none()
    {
        return;
    }
    let cfg = rusty::config();
    let settings = &cfg.awareness;
    OBS.with_borrow(|o| {
        out.lines.extend_from_slice(&o.geometry.lines);
        out.on_top.extend_from_slice(&o.geometry.on_top);
    });
    if settings.chunk_borders {
        let Some(player) = game.native_player() else {
            return;
        };
        if let (Some(x), Some(z)) = (logic::chunk_origin(player.x), logic::chunk_origin(player.z)) {
            let low = (player.y.floor() - 16.).max(f64::from(game.world.bottom_y()));
            let high = (player.y.floor() + 32.).min(f64::from(game.world.top_y()));
            for (dx, dz) in [(0., 0.), (16., 0.), (16., 16.), (0., 16.)] {
                out.line(
                    [f64::from(x) + dx, low, f64::from(z) + dz],
                    [f64::from(x) + dx, high, f64::from(z) + dz],
                    0xffedc77d,
                    2.,
                );
            }
            for y in ((low as i32).div_euclid(16) * 16..=high as i32).step_by(16) {
                let y = f64::from(y);
                for (a, b) in [
                    ([0., 0.], [16., 0.]),
                    ([16., 0.], [16., 16.]),
                    ([16., 16.], [0., 16.]),
                    ([0., 16.], [0., 0.]),
                ] {
                    out.line(
                        [f64::from(x) + a[0], y, f64::from(z) + a[1]],
                        [f64::from(x) + b[0], y, f64::from(z) + b[1]],
                        0xffc69e66,
                        1.,
                    );
                }
            }
        }
    }
    if settings.trajectories {
        trajectory(game, out);
    }
    OBS.with_borrow_mut(|o| o.entity_tracer_lines = 0);
    if settings.entity_tracers {
        let mut lines = Gizmos::default();
        // Starting exactly at the eye makes both projected endpoints coincide.
        // Use the same below-crosshair view-space origin as block tracers;
        // the actual view matrix also follows a detached camera correctly.
        let offset = view
            .transpose()
            .transform_vector3(glam::Vec3::new(0., -0.18, -0.65));
        let start = std::array::from_fn(|i| camera[i] + f64::from(offset[i]));
        let radius = f64::from(cfg.scan_radius);
        for mob in game.mobs.iter().take(4096).filter(|m| !m.removed) {
            if !settings.traces_entity(mob.kind.id == "minecraft:player", false) {
                continue;
            }
            let at = [
                mob.position.x,
                mob.position.y + f64::from(mob.height()) * 0.5,
                mob.position.z,
            ];
            if distance2(camera, at) <= radius * radius {
                lines.line(
                    start,
                    at,
                    if mob.kind.id == "minecraft:player" {
                        0xff7caeff
                    } else {
                        0xffffa36c
                    },
                    1.5,
                );
                if lines.lines.len() >= 128 {
                    break;
                }
            }
        }
        if settings.traces_entity(false, true) {
            for item in game.dropped.iter().take(4096) {
                if lines.lines.len() >= 128 {
                    break;
                }
                let at = [item.position.x, item.position.y + 0.125, item.position.z];
                if distance2(camera, at) <= radius * radius {
                    lines.line(start, at, 0xffb4f28c, 1.5);
                }
            }
        }
        OBS.with_borrow_mut(|o| o.entity_tracer_lines = lines.lines.len());
        out.on_top.extend(lines.lines);
    }
}
pub(super) fn hud(game: &Game<'_>, batch: &mut HudBatch) {
    if !game.native_window()
        || !game.native_on(rusty::ID)
        || game.ui.hide_gui
        || !game.native_playing()
    {
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };
    let cfg = rusty::config();
    let settings = &cfg.awareness;
    let mut lines = Vec::new();
    let mut groups = Vec::new();
    let mut group_start = 0;
    if settings.fps_hud {
        let (fps, min, low) = game.native_frame_rate();
        lines.push(format!("{fps} FPS | min {min} | 1% {low}"));
    }
    groups.push(("fps_hud", group_start, lines.len()));
    group_start = lines.len();
    if settings.clicks_hud {
        OBS.with_borrow(|o| lines.push(format!("Clicks/s L {} / R {}", o.clicks.0, o.clicks.1)));
    }
    groups.push(("clicks_hud", group_start, lines.len()));
    group_start = lines.len();
    if settings.portal_coordinates {
        if let Some((label, at)) =
            logic::portal_position(player.dimension, [player.x, player.y, player.z])
        {
            lines.push(format!("{label} XZ {:.1} / {:.1}", at[0], at[2]));
        } else {
            lines.push("Portal projection unavailable in this dimension".into());
        }
    }
    groups.push(("portal_coordinates", group_start, lines.len()));
    group_start = lines.len();
    if settings.clock_hud {
        if let Some(clock) = game.native_clock() {
            lines.push(format!("World {}", logic::clock_text(clock.day_time)));
        }
    }
    groups.push(("clock_hud", group_start, lines.len()));
    group_start = lines.len();
    if settings.speed_hud {
        OBS.with_borrow(|o| {
            lines.push(o.speed.map_or_else(
                || "Speed --".into(),
                |speed| format!("Speed {speed:.2} m/s"),
            ))
        });
    }
    groups.push(("speed_hud", group_start, lines.len()));
    group_start = lines.len();
    if settings.target_info {
        if let Some(hit) = game.pick() {
            let state = game.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
            let facts = game.res.facts().get(state);
            lines.push(rusty::friendly_name(&facts.name));
            lines.push(format!(
                "Block {} / {} / {}",
                hit.pos[0], hit.pos[1], hit.pos[2]
            ));
            if let Some(eye) = game.eye_position() {
                lines.push(format!(
                    "Distance {:.2} m",
                    distance2(eye, hit.point).sqrt()
                ));
            }
            if let Some(state) = game.pack.assets.registry.state(state) {
                for (key, value) in state.properties.iter().take(4) {
                    lines.push(format!("{key}: {value}"));
                }
            }
        } else {
            lines.push("Target: no received block in reach".into());
        }
    }
    groups.push(("target_info", group_start, lines.len()));
    group_start = lines.len();
    if settings.light_overlay {
        lines.push("Light risk: red dark / amber skylit".into());
    }
    groups.push(("light_overlay", group_start, lines.len()));
    group_start = lines.len();
    if settings.trajectories {
        OBS.with_borrow(|o| {
            lines.push(if o.prediction_kind.is_empty() {
                "Trajectory: hold a supported item".into()
            } else {
                format!(
                    "{} estimate | {}{}",
                    o.prediction_kind,
                    o.prediction_status,
                    if o.prediction_partial {
                        " (partial)"
                    } else {
                        ""
                    }
                )
            });
        });
    }
    groups.push(("trajectories", group_start, lines.len()));
    let mut panels = Vec::new();
    for (panel, start, end) in groups {
        if start == end {
            continue;
        }
        let rows: Vec<_> = lines[start..end]
            .iter()
            .map(|line| (line.clone(), 0xffedddca))
            .collect();
        panels.push((panel, rows));
    }
    super::rusty_hud_native::stacked_lines(game, batch, &cfg.client.hud_layout, &panels);
}

pub(super) fn snapshot() -> serde_json::Value {
    OBS.with_borrow(|o|serde_json::json!({"light_cells":o.light_cells,"storage_boxes":o.storage_boxes,
        "storage_partial":o.storage_partial,"speed":o.speed,"entity_tracer_lines":o.entity_tracer_lines,"clicks":{"left":o.clicks.0,"right":o.clicks.1},
        "prediction":{"segments":o.prediction_count,"kind":o.prediction_kind,"status":o.prediction_status,"partial":o.prediction_partial},"depth_lines":o.geometry.lines.len(),"on_top_lines":o.geometry.on_top.len()}))
}
