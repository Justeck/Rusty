// Shared screen HUD renderer bridge. Copyright 2026 Justeck, Apache-2.0.
// Lives below both horizon-client-lib and gameplay; no crate::game dependency.
use super::rusty_hud_layout::{HudLayout, PANELS, PanelBounds, Resolved};
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
    static PREVIEW: RefCell<Option<HudLayout>> = const { RefCell::new(None) };
    static BOUNDS: RefCell<Vec<PanelBounds>> = RefCell::new(Vec::with_capacity(PANELS.len()));
    static MAP_BOUNDS: Cell<Option<PanelBounds>> = const { Cell::new(None) };
}
pub fn preview(layout: Option<HudLayout>) {
    PREVIEW.with_borrow_mut(|p| *p = layout);
}
pub fn bounds() -> Vec<PanelBounds> {
    BOUNDS.with_borrow(Clone::clone)
}
fn record(id: &'static str, size: [f32; 2], r: Resolved) {
    let panel = PanelBounds {
        id,
        rect: [
            r.origin[0],
            r.origin[1],
            size[0] * r.scale,
            size[1] * r.scale,
        ],
        size,
        scale: r.scale,
    };
    if id == "minimap" {
        MAP_BOUNDS.set(Some(panel));
    }
    BOUNDS.with_borrow_mut(|b| {
        b.retain(|p| p.id != id);
        b.push(panel);
    });
}
pub fn minimap_owned(owned: bool) {
    MINIMAP_OWNED.set(owned);
    MINIMAP_TOP.set(None);
    MAP_BOUNDS.set(None);
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
    BOUNDS.with_borrow_mut(|b| {
        b.clear();
        if let Some(p) = MAP_BOUNDS.get() {
            b.push(p);
        }
    });
    ACTIVE.set(true);
}
pub fn finish(batch: &mut HudBatch, layout: &HudLayout) {
    let edit = PREVIEW.with_borrow(Clone::clone);
    let layout = edit.as_ref().unwrap_or(layout);
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
            if let Some(r) = resolved {
                record(PANELS[panel.id], panel.size, r);
            }
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
    let edit = PREVIEW.with_borrow(Clone::clone);
    let layout = edit.as_ref().unwrap_or(layout);
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
        record("minimap", size, resolved);
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
    if let Some(&id) = PANELS.iter().find(|p| **p == id) {
        record(id, size, resolved);
    }
    apply(batch, start, min, resolved);
}
