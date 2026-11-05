// Rusty native overlays, Copyright 2026 Justeck, Apache-2.0.
// Uses Horizon's public native gizmo/HUD interfaces (Horizon, MIT).
// No resource-pack textures or GPU resources are included by this module.
use super::Game;
use horizon_client::{chunk::Section, gizmos::Gizmos, hud::HudBatch};
use horizon_client_render::mesher::rusty;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

const CACHE_LIMIT: usize = 4096;
const SECTION_TARGET_LIMIT: usize = 4096;
const TARGET_LIMIT: usize = 4096;
const RAW_TARGET_LIMIT: usize = 65536;
const BOX_LIMIT: usize = 512;
const CONTEXT_LIMIT: usize = 2048;
const CACHED_CONTEXT_LIMIT: usize = 32768;
const CONTEXT_CANDIDATE_LIMIT: usize = 8192;
const SECTION_CONTEXT_LIMIT: usize = 2048;
const REBUILD_BUDGET: usize = 4;
type Key = (i32, usize, i32);

#[derive(Clone, Default, Debug)]
pub(super) struct Stats {
    pub total: usize,
    pub per_rule: Vec<usize>,
    pub pending: usize,
    pub truncated: bool,
    pub nearest: Option<([i32; 3], usize, f64)>,
    pub aggregations: u64,
    pub reused: u64,
}
#[derive(Clone, Copy)]
struct Target {
    pos: [i32; 3],
    rule: usize,
    distance: f64,
}
struct Cached {
    source: Arc<Section>,
    neighbors: Vec<Option<Arc<Section>>>,
    targets: Vec<([i32; 3], usize)>,
    faces: Vec<super::rusty_terrain::Face>,
    context: bool,
    truncated: bool,
}
#[derive(Default)]
struct Cache {
    revision: u64,
    dimension: String,
    catalog: Option<Arc<rusty::Catalog>>,
    sections: HashMap<Key, Cached>,
    queue: VecDeque<Key>,
    targets: Vec<Target>,
    context: Vec<super::rusty_terrain::Face>,
    stats: Stats,
    last_scan: Option<std::time::Instant>,
    aggregation: horizon_client_render::mesher::rusty_scan_cache::Gate,
}
static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
static SCAN: AtomicBool = AtomicBool::new(true);
fn cache() -> &'static Mutex<Cache> {
    CACHE.get_or_init(|| Mutex::new(Cache::default()))
}
pub(super) fn stats() -> Stats {
    cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .stats
        .clone()
}
pub(super) fn request_scan() {
    SCAN.store(true, Ordering::Release);
}
pub(super) fn reset() {
    *cache().lock().unwrap_or_else(|e| e.into_inner()) = Cache::default();
    request_scan();
}
fn section(game: &Game<'_>, k: Key) -> Option<Arc<Section>> {
    game.world
        .chunks
        .get(&(k.0, k.2))?
        .sections
        .get(k.1)
        .cloned()
}
fn neighbors(game: &Game<'_>, k: Key) -> Vec<Option<Arc<Section>>> {
    let (x, y, z) = k;
    vec![
        section(game, (x - 1, y, z)),
        section(game, (x + 1, y, z)),
        y.checked_sub(1).and_then(|y| section(game, (x, y, z))),
        section(game, (x, y + 1, z)),
        section(game, (x, y, z - 1)),
        section(game, (x, y, z + 1)),
    ]
}
fn same_neighbors(a: &[Option<Arc<Section>>], b: &[Option<Arc<Section>>]) -> bool {
    a.iter().zip(b).all(|(a, b)| match (a, b) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    })
}
fn distance(pos: [i32; 3], at: [f64; 3]) -> f64 {
    (0..3)
        .map(|i| (f64::from(pos[i]) + 0.5 - at[i]).powi(2))
        .sum::<f64>()
        .sqrt()
}
fn context_section(k: Key, min_y: i32, at: [f64; 3], radius: u32) -> bool {
    let low = [k.0 * 16, min_y + k.1 as i32 * 16, k.2 * 16];
    (0..3)
        .map(|i| {
            let nearest = at[i].clamp(f64::from(low[i]), f64::from(low[i] + 16));
            (nearest - at[i]).powi(2)
        })
        .sum::<f64>()
        <= f64::from(radius.min(48)).powi(2)
}
pub(super) fn selected(cat: &rusty::Catalog, cfg: &rusty::Config, state: u32, y: i32) -> bool {
    cat.matches
        .get(state as usize)
        .copied()
        .flatten()
        .and_then(|i| cfg.rules.get(i))
        .is_some_and(|r| r.enabled && y >= r.min_y && y <= r.max_y)
}
fn rebuild(
    game: &Game<'_>,
    k: Key,
    source: Arc<Section>,
    cfg: &rusty::Config,
    cat: &rusty::Catalog,
    context: bool,
) -> Cached {
    let mut result = Cached {
        source,
        neighbors: neighbors(game, k),
        targets: Vec::new(),
        faces: Vec::new(),
        context,
        truncated: false,
    };
    if result.source.is_uniform(0) {
        return result;
    }
    if !context
        && result.source.blocks.uniform().is_some_and(|state| {
            cat.matches
                .get(state as usize)
                .copied()
                .flatten()
                .and_then(|i| cfg.rules.get(i))
                .is_none_or(|rule| !rule.enabled)
        })
    {
        return result;
    }
    let mut blocks = [0u32; 4096];
    result.source.unpack(&mut blocks);
    let min_y = game.world.bottom_y();
    let base = [k.0 * 16, min_y + k.1 as i32 * 16, k.2 * 16];
    let facts = game.res.facts();
    for (i, &state) in blocks.iter().enumerate() {
        let local = [(i & 15) as i32, (i >> 8) as i32, ((i >> 4) & 15) as i32];
        let pos = std::array::from_fn(|j| base[j] + local[j]);
        if let Some(rule) = cat.matches.get(state as usize).copied().flatten()
            && cfg
                .rules
                .get(rule)
                .is_some_and(|r| r.enabled && pos[1] >= r.min_y && pos[1] <= r.max_y)
        {
            if result.targets.len() < SECTION_TARGET_LIMIT {
                result.targets.push((pos, rule));
            } else {
                result.truncated = true;
            }
        }
        if !context
            || state == 0
            || selected(cat, cfg, state, pos[1])
            || facts
                .states
                .get(state as usize)
                .is_none_or(|s| s.render_shape != horizon_client::blocks::RenderShape::Model)
        {
            continue;
        }
        if result.faces.len() < SECTION_CONTEXT_LIMIT {
            let mut faces = super::rusty_terrain::faces(game, pos, state, cfg, cat);
            let room = SECTION_CONTEXT_LIMIT - result.faces.len();
            if faces.len() > room {
                faces.truncate(room);
                result.truncated = true;
            }
            result.faces.extend(faces);
        } else {
            result.truncated = true;
        }
    }
    result
}

pub(super) fn frame(game: &Game<'_>) {
    if !game.native_window() || !game.native_on(rusty::ID) {
        return;
    }
    let Some(player) = game.native_player() else {
        reset();
        return;
    };
    let cfg = rusty::config();
    let cat = rusty::catalog();
    let at = [player.x, player.y, player.z];
    let mut c = cache().lock().unwrap_or_else(|e| e.into_inner());
    if c.revision != rusty::revision()
        || c.dimension != player.dimension
        || c.catalog.as_ref().is_none_or(|old| !Arc::ptr_eq(old, &cat))
    {
        *c = Cache {
            revision: rusty::revision(),
            dimension: player.dimension.into(),
            catalog: Some(cat.clone()),
            ..Cache::default()
        };
        SCAN.store(true, Ordering::Release);
    }
    let enabled = (rusty::active() && !cfg.normal_terrain()) || cfg.finder_enabled;
    if !enabled {
        c.targets.clear();
        c.context.clear();
        c.stats = Stats::default();
        c.aggregation.clear();
        return;
    }
    let min_y = game.world.bottom_y();
    let scan_radius = cfg.scan_radius.clamp(1, 128);
    let context_on = rusty::active() && cfg.context_outlines && !cfg.normal_terrain();
    let radius = scan_radius.max(if context_on { cfg.context_radius } else { 0 });
    let previous_sections = c.sections.len();
    let mut changed = false;
    c.sections.retain(|&k, entry| {
        let dx = f64::from(k.0 * 16 + 8) - at[0];
        let dz = f64::from(k.2 * 16 + 8) - at[2];
        if dx.abs() > f64::from(radius) + 16. || dz.abs() > f64::from(radius) + 16. {
            return false;
        }
        section(game, k).is_some_and(|now| Arc::ptr_eq(&entry.source, &now))
    });
    changed |= previous_sections != c.sections.len();
    // Drop terrain geometry outside its navigation radius before allocating
    // newly entered sections. Target records retain their independent radius.
    for (&key, entry) in c.sections.iter_mut() {
        if entry.context && (!context_on || !context_section(key, min_y, at, cfg.context_radius)) {
            changed = true;
            entry.faces = Vec::new();
            entry.context = false;
        }
    }
    let manual = SCAN.swap(false, Ordering::AcqRel);
    if manual
        || cfg.auto_scan
            && c.last_scan.is_none_or(|t| {
                t.elapsed()
                    >= std::time::Duration::from_millis(u64::from(cfg.scan_interval.max(1)) * 50)
            })
    {
        c.last_scan = Some(std::time::Instant::now());
        let mut desired = Vec::new();
        let cx = (player.x.floor() as i32) >> 4;
        let cz = (player.z.floor() as i32) >> 4;
        let columns = ((radius + 15) / 16) as i32;
        let rule_min = cfg
            .rules
            .iter()
            .filter(|r| r.enabled)
            .map(|r| r.min_y)
            .min()
            .unwrap_or(min_y);
        let rule_max = cfg
            .rules
            .iter()
            .filter(|r| r.enabled)
            .map(|r| r.max_y)
            .max()
            .unwrap_or(min_y - 1);
        for z in cz - columns..=cz + columns {
            for x in cx - columns..=cx + columns {
                let Some(chunk) = game.world.chunks.get(&(x, z)) else {
                    continue;
                };
                for sy in 0..chunk.sections.len() {
                    let k = (x, sy, z);
                    let y = min_y + sy as i32 * 16;
                    let context = context_on && context_section(k, min_y, at, cfg.context_radius);
                    if !context && (y > rule_max || y + 15 < rule_min) {
                        continue;
                    }
                    let need = c.sections.get(&k).is_none_or(|old| {
                        old.context != context
                            || context && !same_neighbors(&old.neighbors, &neighbors(game, k))
                    });
                    if need {
                        desired.push((distance([x * 16 + 8, y + 8, z * 16 + 8], at), k));
                    }
                }
            }
        }
        desired.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        c.queue = desired
            .into_iter()
            .take(CACHE_LIMIT)
            .map(|(_, k)| k)
            .collect();
    }
    for _ in 0..REBUILD_BUDGET {
        let Some(k) = c.queue.pop_front() else {
            break;
        };
        if c.sections.len() >= CACHE_LIMIT && !c.sections.contains_key(&k) {
            break;
        }
        if let Some(source) = section(game, k) {
            let context = context_on && context_section(k, min_y, at, cfg.context_radius);
            let used = c
                .sections
                .iter()
                .filter(|(key, _)| **key != k)
                .map(|(_, s)| s.targets.len())
                .sum::<usize>();
            let cached_faces: usize = c
                .sections
                .iter()
                .filter(|(key, _)| **key != k)
                .map(|(_, s)| s.faces.len())
                .sum();
            let context_room = CACHED_CONTEXT_LIMIT.saturating_sub(cached_faces);
            let mut rebuilt = rebuild(game, k, source, &cfg, &cat, context && context_room > 0);
            if rebuilt.faces.len() > context_room {
                rebuilt.faces.truncate(context_room);
                rebuilt.truncated = true;
            }
            if context && context_room == 0 {
                rebuilt.truncated = true;
            }

            let room = RAW_TARGET_LIMIT.saturating_sub(used);
            if rebuilt.targets.len() > room {
                rebuilt.targets.truncate(room);
                rebuilt.truncated = true;
            }
            c.sections.insert(k, rebuilt);
            changed = true;
        }
    }
    // Chunk validation and the bounded rebuild queue always run above. Only
    // derived distance/filter/sort buffers are reused for identical inputs.
    let stamp = horizon_client_render::mesher::rusty_scan_cache::Stamp {
        position: at.map(f64::to_bits),
        config: rusty::config_serial(),
        context: context_on,
    };
    if !c.aggregation.aggregate(stamp, changed) {
        c.stats.pending = c.queue.len();
        c.stats.reused = c.stats.reused.saturating_add(1);
        return;
    }
    let mut targets = Vec::new();
    let mut per_rule = vec![0; cfg.rules.len()];
    let mut truncated = c.sections.len() >= CACHE_LIMIT || c.sections.values().any(|v| v.truncated);
    let mut total = 0usize;
    for entry in c.sections.values() {
        for &(pos, rule) in &entry.targets {
            if let Some(r) = cfg.rules.get(rule)
                && r.contains(pos, at)
            {
                let dx = f64::from(pos[0]) + 0.5 - at[0];
                let dz = f64::from(pos[2]) + 0.5 - at[2];
                if dx * dx + dz * dz > f64::from(scan_radius).powi(2) {
                    continue;
                }
                per_rule[rule] += 1;
                total += 1;
                // Bound temporary memory even in intentionally dense selections.
                if targets.len() < 65536 {
                    targets.push(Target {
                        pos,
                        rule,
                        distance: distance(pos, at),
                    });
                } else {
                    truncated = true;
                }
            }
        }
    }
    targets.sort_by(|a, b| a.distance.total_cmp(&b.distance).then(a.pos.cmp(&b.pos)));
    let max = (cfg.max_targets as usize).clamp(1, TARGET_LIMIT);
    truncated |= targets.len() > max;
    targets.truncate(max);
    let nearest = targets.first().map(|t| (t.pos, t.rule, t.distance));
    let mut context = Vec::new();
    if context_on {
        let mut ordered: Vec<_> = c.sections.iter().filter(|(_, s)| s.context).collect();
        ordered.sort_by(|(a, _), (b, _)| {
            distance(
                [a.0 * 16 + 8, min_y + a.1 as i32 * 16 + 8, a.2 * 16 + 8],
                at,
            )
            .total_cmp(&distance(
                [b.0 * 16 + 8, min_y + b.1 as i32 * 16 + 8, b.2 * 16 + 8],
                at,
            ))
            .then(a.cmp(b))
        });
        let mut faces = Vec::new();
        'sections: for (_, entry) in ordered {
            for face in &entry.faces {
                let d2: f64 = (0..3)
                    .map(|i| (f64::from(face.pos[i]) + 0.5 - at[i]).powi(2))
                    .sum();
                if d2 > f64::from(cfg.context_radius).powi(2) {
                    continue;
                }
                faces.push((d2, face));
                if faces.len() >= CONTEXT_CANDIDATE_LIMIT {
                    truncated = true;
                    break 'sections;
                }
            }
        }
        faces.sort_by(|(a, f), (b, g)| a.total_cmp(b).then(f.pos.cmp(&g.pos)));
        truncated |= faces.len() > CONTEXT_LIMIT;
        context.extend(
            faces
                .into_iter()
                .take(CONTEXT_LIMIT)
                .map(|(_, f)| f.clone()),
        );
    }
    c.stats = Stats {
        aggregations: c.stats.aggregations.saturating_add(1),
        reused: c.stats.reused,
        total,
        per_rule,
        pending: c.queue.len(),
        truncated,
        nearest,
    };
    c.targets = targets;
    c.context = context;
}

fn push_box(g: &mut Gizmos, at: [f64; 3], size: [f64; 3], color: u32, width: f32) {
    let max = std::array::from_fn(|i| at[i] + size[i]);
    g.cuboid(at, max, color, width);
}
fn line_gizmos(game: &Game<'_>, camera: [f64; 3], view: glam::Mat4, on_top: bool) -> Gizmos {
    if !game.native_window()
        || !game.native_on(rusty::ID)
        || game.native_player().is_none()
        || game.ui.hide_gui
    {
        return Gizmos::default();
    }
    let cfg = rusty::config();
    let mut g = Gizmos::default();
    if !on_top && cfg.navigation.breadcrumbs_enabled {
        for [a, b] in super::rusty_navigation_native::segments(game) {
            g.line(a, b, 0xffdca56f, 1.5);
        }
    }
    // Finder owns target geometry; X-ray only changes terrain rendering.
    if cfg.finder_enabled {
        let c = cache().lock().unwrap_or_else(|e| e.into_inner());
        if on_top {
            let width = cfg.line_width.clamp(0.5, 4.0);
            if cfg.block_outlines {
                for t in c.targets.iter().take(BOX_LIMIT) {
                    if let Some(rule) = cfg.rules.get(t.rule)
                        && rule.outline
                    {
                        push_box(&mut g, t.pos.map(f64::from), [1.; 3], rule.color, width);
                    }
                }
            }
            if cfg.tracers && cfg.tracer_limit > 0 {
                let forward = view
                    .transpose()
                    .transform_vector3(glam::Vec3::new(0., -0.18, -0.65));
                let start = std::array::from_fn(|i| camera[i] + f64::from(forward[i]));
                let mut groups = HashSet::new();
                let mut count = 0;
                for t in &c.targets {
                    let Some(rule) = cfg.rules.get(t.rule) else {
                        continue;
                    };
                    // One representative per rule and four-block cell reduces vein clutter.
                    if !rule.tracers
                        || !groups.insert((
                            t.rule,
                            t.pos[0].div_euclid(4),
                            t.pos[1].div_euclid(4),
                            t.pos[2].div_euclid(4),
                        ))
                    {
                        continue;
                    }
                    let end = t.pos.map(|p| f64::from(p) + 0.5);
                    if (0..3).map(|i| (end[i] - start[i]).powi(2)).sum::<f64>() < 0.04 {
                        continue;
                    }
                    g.line(start, end, rule.color, width);
                    count += 1;
                    if count >= cfg.tracer_limit.clamp(1, 12) {
                        break;
                    }
                }
            }
        }
    }
    if on_top && cfg.entity_esp {
        entity_lines(game, &cfg, camera, &mut g);
    }
    g
}
pub(super) fn append_gizmos(
    mut value: Gizmos,
    game: &Game<'_>,
    camera: [f64; 3],
    view: glam::Mat4,
) -> Gizmos {
    value
        .lines
        .extend(line_gizmos(game, camera, view, false).lines);
    value
        .on_top
        .extend(line_gizmos(game, camera, view, true).lines);
    super::rusty_awareness_native::gizmos(game, &mut value, camera, view);
    super::rusty_builder::append_gizmos(game, &mut value);
    value
}
fn entity_lines(game: &Game<'_>, cfg: &rusty::Config, camera: [f64; 3], g: &mut Gizmos) {
    let alpha = f64::from(game.clock.camera_fraction() as f32);
    let radius = f64::from(cfg.scan_radius.clamp(1, 128));
    let mut count = 0usize;
    // Window clients receive player models through the ordinary remote entity path.
    for m in game.mobs.iter().take(4096) {
        let player = m.kind.id == "minecraft:player";
        if m.removed || player && !cfg.esp_players || !player && !cfg.esp_mobs {
            continue;
        }
        let pos = [
            m.previous.x + (m.position.x - m.previous.x) * alpha,
            m.previous.y + (m.position.y - m.previous.y) * alpha,
            m.previous.z + (m.position.z - m.previous.z) * alpha,
        ];
        if (0..3).map(|i| (pos[i] - camera[i]).powi(2)).sum::<f64>() > radius * radius {
            continue;
        }
        let w = f64::from(m.width());
        push_box(
            g,
            [pos[0] - w * 0.5, pos[1], pos[2] - w * 0.5],
            [w, f64::from(m.height()), w],
            if player { 0xff7caeff } else { 0xffffa36c },
            cfg.line_width.clamp(0.5, 4.),
        );
        count += 1;
        if count >= 128 {
            break;
        }
    }
    if cfg.esp_items {
        for item in game.dropped.iter().take(4096) {
            let pos = [
                item.previous.x + (item.position.x - item.previous.x) * alpha,
                item.previous.y + (item.position.y - item.previous.y) * alpha,
                item.previous.z + (item.position.z - item.previous.z) * alpha,
            ];
            if (0..3).map(|i| (pos[i] - camera[i]).powi(2)).sum::<f64>() > radius * radius {
                continue;
            }
            push_box(
                g,
                [pos[0] - 0.125, pos[1], pos[2] - 0.125],
                [0.25; 3],
                0xffb4f28c,
                1.,
            );
            count += 1;
            if count >= 256 {
                break;
            }
        }
    }
}
pub(super) fn hud(game: &Game<'_>, batch: &mut HudBatch) {
    if !game.native_window() || !game.native_on(rusty::ID) || game.ui.hide_gui {
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };
    let cfg = rusty::config();
    if !cfg.hud && !cfg.coords_hud {
        return;
    }
    let s = stats();
    let mut lines = Vec::<(String, u32)>::new();
    let finder_start = batch.vertices.len();
    let mut finder_lines = Vec::new();
    if cfg.hud && cfg.finder_enabled {
        let mode = "Finder";
        let pending = if s.pending > 0 {
            format!(" | {} pending", s.pending)
        } else {
            String::new()
        };
        finder_lines.push((
            format!(
                "Rusty {mode} | {}{}{}",
                s.total,
                if s.truncated { "+" } else { "" },
                pending
            ),
            0xffb7dce4,
        ));
        if cfg.hud_counts {
            for (i, &n) in s
                .per_rule
                .iter()
                .enumerate()
                .filter(|(_, n)| **n > 0)
                .take(5)
            {
                if let Some(rule) = cfg.rules.get(i) {
                    finder_lines.push((format!("{}: {n}", rule.display_name()), rule.color));
                }
            }
        }
        if let Some((pos, rule, d)) = s.nearest {
            let target_yaw = (-(f64::from(pos[0]) + 0.5 - player.x))
                .atan2(f64::from(pos[2]) + 0.5 - player.z)
                .to_degrees();
            let bearing = (target_yaw - f64::from(player.yaw) + 180.).rem_euclid(360.) - 180.;
            let direction = if bearing.abs() < 12. {
                "ahead"
            } else if bearing.abs() > 160. {
                "behind"
            } else if bearing < 0. {
                "left"
            } else {
                "right"
            };
            let name = cfg
                .rules
                .get(rule)
                .map_or_else(|| "Target".into(), |r| r.display_name());
            finder_lines.push((
                format!("{name} {d:.0}m {direction} | Y {}", pos[1]),
                0xffe1ecef,
            ));
            // A small compass pointer remains meaningful for offscreen targets.
            let a = bearing.to_radians();
            let center = [10.0, 10.0];
            let tip = [
                center[0] + a.sin() as f32 * 9.,
                center[1] - a.cos() as f32 * 9.,
            ];
            let side = [a.cos() as f32 * 3., a.sin() as f32 * 3.];
            let tail = [
                center[0] - a.sin() as f32 * 3.,
                center[1] + a.cos() as f32 * 3.,
            ];
            let corners = [
                tip,
                [tail[0] + side[0], tail[1] + side[1]],
                tail,
                [tail[0] - side[0], tail[1] - side[1]],
            ];
            let argb = cfg.rules.get(rule).map_or(0xffb7dce4, |r| r.color);
            let [a, r, g, b] = argb.to_be_bytes();
            batch.fill_quad(
                &game.gui.atlas,
                corners,
                horizon_client::hud::rgba(r, g, b, a),
            );
        }
    }
    super::rusty_hud_native::draw_lines_at(game, batch, &finder_lines, [24.0, 0.0]);
    super::rusty_hud_native::place(batch, finder_start, &cfg.client.hud_layout, "finder");
    if cfg.hud {
        if rusty::active() {
            lines.push(("Xray".into(), 0xffb7dce4));
        }
        let mut active = Vec::new();
        for (on, name) in [
            (cfg.assistance.safe_walk, "Safe Walk"),
            (cfg.assistance.auto_jump, "Auto Jump"),
            (cfg.assistance.auto_tool, "Auto Tool"),
            (cfg.assistance.trigger_bot, "Trigger Bot"),
            (
                cfg.assistance.auto_eat,
                if super::rusty_assist::eating_paused() {
                    "Auto Eat (paused)"
                } else {
                    "Auto Eat"
                },
            ),
            (cfg.assistance.auto_fish, "Auto Fish"),
            (cfg.assistance.auto_totem, "Auto Totem"),
            (cfg.assistance.armor_assist, "Armor Assist"),
            (cfg.assistance.refill, "Auto Refill"),
        ] {
            if on {
                active.push(name);
            }
        }
        for row in active.chunks(3) {
            lines.push((row.join(" | "), 0xffdcc19b));
        }
    }
    super::rusty_hud_native::lines(game, batch, &cfg.client.hud_layout, "status_hud", &lines);
    if cfg.coords_hud {
        let coordinates = vec![(
            format!(
                "XYZ {:.1} / {:.1} / {:.1} | {}",
                player.x,
                player.y,
                player.z,
                player
                    .dimension
                    .strip_prefix("minecraft:")
                    .unwrap_or(player.dimension)
            ),
            0xffdddddd,
        )];
        super::rusty_hud_native::lines(
            game,
            batch,
            &cfg.client.hud_layout,
            "coordinates",
            &coordinates,
        );
    }
}

pub(super) fn append_terrain(
    mut value: Vec<horizon_client::mesher::Vertex>,
    game: &Game<'_>,
    camera: [f64; 3],
) -> Vec<horizon_client::mesher::Vertex> {
    let cfg = rusty::config();
    if !game.native_window()
        || !game.native_on(rusty::ID)
        || !rusty::active()
        || !cfg.context_outlines
        || cfg.normal_terrain()
        || cfg.context_opacity == 0
        || game.native_player().is_none()
    {
        return value;
    }
    let c = cache().lock().unwrap_or_else(|e| e.into_inner());
    for face in &c.context {
        super::rusty_terrain::append_face(&mut value, face, camera, &cfg);
    }
    value
}
