// Shared screen HUD renderer bridge. Copyright 2026 Justeck, Apache-2.0.
// Lives below both horizon-client-lib and gameplay; no crate::game dependency.
use super::rusty_hud_layout::{HudLayout, PANELS, Resolved};
use crate::hud::HudBatch;
use std::cell::{Cell, RefCell};
#[derive(Clone, Copy)]
struct Panel {
    id: usize,
    start: usize,
    end: usize,
    source: [f32; 2],
    size: [f32; 2],
}
thread_local! {
    static MINIMAP_OWNED: Cell<bool> = const { Cell::new(false) };
    static MINIMAP_START: Cell<Option<usize>> = const { Cell::new(None) };
    static MINIMAP_TOP: Cell<Option<([f32; 2], f32)>> = const { Cell::new(None) };
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static PANELS_DRAWN: RefCell<Vec<Panel>> = RefCell::new(Vec::with_capacity(PANELS.len()));
}
pub fn minimap_owned(owned: bool) {
    MINIMAP_OWNED.set(owned);
    MINIMAP_TOP.set(None);
}
pub fn minimap_begin(batch: &HudBatch) {
    MINIMAP_START.set(MINIMAP_OWNED.get().then_some(batch.vertices.len()));
}
pub fn minimap_end(batch: &mut HudBatch) {
    if let Some(start) = MINIMAP_START.take() {
        place(
            batch,
            start,
            &super::rusty::config().client.hud_layout,
            "minimap",
        );
    }
}
pub fn begin() {
    PANELS_DRAWN.with_borrow_mut(Vec::clear);
    ACTIVE.set(true);
}
pub fn finish(batch: &mut HudBatch, layout: &HudLayout) {
    ACTIVE.set(false);
    PANELS_DRAWN.with_borrow_mut(|panels| {
        let sizes: Vec<_> = panels.iter().map(|p| (PANELS[p.id], p.size)).collect();
        let viewport = [batch.width, batch.height];
        let map_top = MINIMAP_TOP
            .get()
            .filter(|(size, _)| *size == viewport)
            .map(|(_, top)| top);
        let resolved = layout.resolve_default_columns(viewport, &sizes, map_top);
        for (panel, resolved) in panels.iter().zip(resolved) {
            if let Some(vertices) = batch.vertices.get_mut(panel.start..panel.end) {
                for vertex in vertices {
                    vertex.pos =
                        resolved.map_or([-1.0, -1.0], |r| r.point(vertex.pos, panel.source));
                }
            }
        }
        panels.clear();
    });
}
fn apply(batch: &mut HudBatch, start: usize, source: [f32; 2], resolved: Resolved) {
    for vertex in &mut batch.vertices[start..] {
        vertex.pos = resolved.point(vertex.pos, source);
    }
}
pub fn place(batch: &mut HudBatch, start: usize, layout: &HudLayout, id: &str) {
    let Some(vertices) = batch.vertices.get(start..) else {
        return;
    };
    if vertices.is_empty() {
        return;
    }
    let mut min = [f32::INFINITY; 2];
    let mut max = [f32::NEG_INFINITY; 2];
    for vertex in vertices {
        for axis in 0..2 {
            min[axis] = min[axis].min(vertex.pos[axis]);
            max[axis] = max[axis].max(vertex.pos[axis]);
        }
    }
    let size = [max[0] - min[0], max[1] - min[1]];
    let viewport = [batch.width, batch.height];
    let Some(resolved) = layout.get(id).resolve(viewport, size) else {
        return;
    };
    if id == "minimap" {
        if !layout.0.contains_key(id) {
            MINIMAP_TOP.set(Some((viewport, resolved.origin[1])));
        }
        apply(batch, start, min, resolved);
        return;
    }
    if ACTIVE.get()
        && !layout.0.contains_key(id)
        && let Some(index) = PANELS.iter().position(|p| *p == id)
    {
        let queued = PANELS_DRAWN.with_borrow_mut(|panels| {
            if panels.len() >= PANELS.len() || panels.iter().any(|p| p.id == index) {
                return false;
            }
            panels.push(Panel {
                id: index,
                start,
                end: batch.vertices.len(),
                source: min,
                size,
            });
            true
        });
        if queued {
            return;
        }
    }
    apply(batch, start, min, resolved);
}
