// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Bounded, conservative mining routes over received terrain. No game inputs here.
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::BinaryHeap};

pub type Node = [i32; 3];

/// Normal walking must stop applying forward input at a reached column while
/// landing. The host's general arrival tolerance also accepts mid-air drops.
pub fn brake_before_landing(
    position: [f64; 3],
    target: [f64; 3],
    near: f64,
    grounded: bool,
) -> bool {
    let horizontal = (position[0] - target[0]).hypot(position[2] - target[2]);
    horizontal <= near && (!grounded || (position[1] - target[1]).abs() > 0.15)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub min: Node,
    pub max: Node,
}
impl Region {
    pub fn new(a: Node, b: Node) -> Result<Self, String> {
        let min = std::array::from_fn(|i| a[i].min(b[i]));
        let max = std::array::from_fn(|i| a[i].max(b[i]));
        if (0..3).any(|i| i64::from(max[i]) - i64::from(min[i]) >= 32)
            || [0, 2]
                .into_iter()
                .any(|i| min[i] < -29_999_984 || max[i] > 29_999_983)
            || min[1] < -2048
            || max[1] > 2047
        {
            return Err(
                "Selection must fit 32 blocks per axis inside the supported world bounds".into(),
            );
        }
        Ok(Self { min, max })
    }
    pub fn contains(self, p: Node) -> bool {
        (0..3).all(|i| p[i] >= self.min[i] && p[i] <= self.max[i])
    }
    pub fn within_border(self, bounds: [f64; 4]) -> bool {
        bounds.iter().all(|v| v.is_finite())
            && f64::from(self.min[0]) >= bounds[0]
            && f64::from(self.min[2]) >= bounds[1]
            && f64::from(self.max[0]) + 1.0 <= bounds[2]
            && f64::from(self.max[2]) + 1.0 <= bounds[3]
    }
    pub fn volume(self) -> usize {
        (0..3)
            .map(|i| (self.max[i] - self.min[i] + 1) as usize)
            .product()
    }
    pub fn position(self, i: usize) -> Option<Node> {
        if i >= self.volume() {
            return None;
        }
        let w = (self.max[0] - self.min[0] + 1) as usize;
        let d = (self.max[2] - self.min[2] + 1) as usize;
        Some([
            self.min[0] + (i % w) as i32,
            self.min[1] + (i / (w * d)) as i32,
            self.min[2] + (i / w % d) as i32,
        ])
    }
    pub fn constrain(self, cell: Cell, p: Node, layer: Option<i32>) -> Cell {
        if self.contains(p) && layer == Some(p[1]) {
            cell
        } else {
            Cell {
                // Outside attachments and tall-block halves may break when their
                // support is removed. Protect neighboring excavation conservatively.
                flags: (cell.flags & !(MINE | RESOURCE))
                    | if !self.contains(p)
                        && cell.flags & KNOWN != 0
                        && cell.flags & (AIR | SOLID) == 0
                    {
                        SUPPORT_GUARD
                    } else {
                        0
                    },
                ..cell
            }
        }
    }
    pub fn constrain_border(
        self,
        cell: Cell,
        p: Node,
        layer: Option<i32>,
        bounds: [f64; 4],
    ) -> Cell {
        // Only known air becomes a route wall. Preserve unknown/hazard and
        // attachment facts beyond the border before testing nearby excavation.
        if cell.is_air() && !(Self { min: p, max: p }).within_border(bounds) {
            Cell::wall()
        } else {
            self.constrain(cell, p, layer)
        }
    }
    pub fn edges(self) -> Vec<[[f64; 3]; 2]> {
        let mut out = Vec::with_capacity(12);
        for axis in 0..3 {
            for a in 0..2 {
                for b in 0..2 {
                    let mut from = self.min.map(f64::from);
                    from[(axis + 1) % 3] = if a == 0 {
                        f64::from(self.min[(axis + 1) % 3])
                    } else {
                        f64::from(self.max[(axis + 1) % 3]) + 1.0
                    };
                    from[(axis + 2) % 3] = if b == 0 {
                        f64::from(self.min[(axis + 2) % 3])
                    } else {
                        f64::from(self.max[(axis + 2) % 3]) + 1.0
                    };
                    let mut to = from;
                    to[axis] = f64::from(self.max[axis]) + 1.0;
                    out.push([from, to]);
                }
            }
        }
        out
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClearSettings {
    pub min_health: f32,
    pub min_food: u32,
    pub min_durability: u8,
    pub preview: bool,
}
impl Default for ClearSettings {
    fn default() -> Self {
        Self {
            min_health: 12.0,
            min_food: 10,
            min_durability: 10,
            preview: true,
        }
    }
}
impl ClearSettings {
    pub fn mining(&self) -> Settings {
        Settings {
            radius: 32,
            tunnels: false,
            min_health: self.min_health,
            min_food: self.min_food,
            min_durability: self.min_durability,
            preview: self.preview,
            ..Settings::default()
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        self.mining().validate()
    }
}
#[derive(Default, Debug)]
pub struct ClearSurvey {
    pub remaining: u32,
    pub protected: u32,
    pub unknown: u32,
    pub dangerous: u32,
    pub highest: Option<i32>,
}
impl ClearSurvey {
    pub fn observe(&mut self, p: Node, cell: Cell) {
        if cell.flags & KNOWN == 0 {
            self.unknown += 1;
        } else if !cell.is_air() {
            self.remaining += 1;
            if cell.flags & DANGER != 0 {
                self.dangerous += 1;
            }
            if cell.flags & MINE != 0 {
                self.highest = Some(self.highest.map_or(p[1], |y| y.max(p[1])));
            } else {
                self.protected += 1;
            }
        }
    }
    pub fn complete(&self) -> bool {
        self.remaining == 0 && self.unknown == 0
    }
}
pub const RESOURCES: [&str; 12] = [
    "Coal",
    "Iron",
    "Copper",
    "Gold",
    "Redstone",
    "Lapis",
    "Diamond",
    "Emerald",
    "Quartz",
    "Nether gold",
    "Ancient debris",
    "Custom",
];
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub resource: u8,
    pub custom_block: String,
    pub radius: u32,
    pub goal: u32,
    pub min_health: f32,
    pub min_food: u32,
    pub min_durability: u8,
    pub tunnels: bool,
    pub preview: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            resource: 6,
            custom_block: "minecraft:diamond_ore".into(),
            radius: 24,
            goal: 16,
            min_health: 12.,
            min_food: 10,
            min_durability: 10,
            tunnels: true,
            preview: true,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        let valid_id = self
            .custom_block
            .split_once(':')
            .is_some_and(|(namespace, path)| {
                !namespace.is_empty()
                    && !path.is_empty()
                    && namespace.bytes().all(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c)
                    })
                    && path.bytes().all(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || b"_/.-".contains(&c)
                    })
            });
        if self.resource > 11
            || !valid_id
            || self.custom_block.len() > 256
            || !(8..=32).contains(&self.radius)
            || !(1..=256).contains(&self.goal)
            || !self.min_health.is_finite()
            || !(1.0..=20.0).contains(&self.min_health)
            || self.min_food > 20
            || !(1..=100).contains(&self.min_durability)
        {
            return Err("Invalid Auto Mine settings".into());
        }
        Ok(())
    }
    pub fn matches(&self, id: &str) -> bool {
        if self.resource == 11 {
            return id == self.custom_block;
        }
        let names = [
            "coal_ore",
            "iron_ore",
            "copper_ore",
            "gold_ore",
            "redstone_ore",
            "lapis_ore",
            "diamond_ore",
            "emerald_ore",
            "nether_quartz_ore",
            "nether_gold_ore",
            "ancient_debris",
        ];
        names.get(usize::from(self.resource)).is_some_and(|name| {
            id.strip_prefix("minecraft:").is_some_and(|s| {
                s == *name || (self.resource < 8 && s.strip_prefix("deepslate_") == Some(*name))
            })
        })
    }
}
#[derive(Clone, Debug)]
pub struct Context {
    pub manual: bool,
    pub damaged: bool,
    pub same_world: bool,
    pub health: f32,
    pub food: u32,
    pub inventory_space: bool,
    pub focused: bool,
    pub playing: bool,
}
impl Default for Context {
    fn default() -> Self {
        Self {
            manual: false,
            damaged: false,
            same_world: true,
            health: 20.,
            food: 20,
            inventory_space: true,
            focused: true,
            playing: true,
        }
    }
}
pub fn guard(settings: &Settings, c: &Context) -> Option<&'static str> {
    if !c.same_world || !c.playing {
        Some("World or player changed")
    } else if !c.focused {
        Some("Window lost focus")
    } else if c.manual {
        Some("Manual input took over")
    } else if c.damaged {
        Some("Player took damage")
    } else if !c.health.is_finite() || c.health < settings.min_health {
        Some("Health is below the safety limit")
    } else if c.food < settings.min_food {
        Some("Food is below the safety limit")
    } else if !c.inventory_space {
        Some("Inventory is full")
    } else {
        None
    }
}
const KNOWN: u8 = 1;
const AIR: u8 = 2;
const SOLID: u8 = 4;
const MINE: u8 = 8;
const RESOURCE: u8 = 16;
const DANGER: u8 = 32;
const SUPPORT_GUARD: u8 = 64;
#[derive(Clone, Copy, Debug, Default)]
pub struct Cell {
    flags: u8,
    ticks: u16,
}
impl Cell {
    pub const fn unknown() -> Self {
        Self { flags: 0, ticks: 0 }
    }
    pub const fn air() -> Self {
        Self {
            flags: KNOWN | AIR,
            ticks: 0,
        }
    }
    pub const fn wall() -> Self {
        Self {
            flags: KNOWN | SOLID,
            ticks: 0,
        }
    }
    pub const fn fluid() -> Self {
        Self {
            flags: KNOWN | DANGER,
            ticks: 0,
        }
    }
    pub const fn falling() -> Self {
        Self {
            flags: KNOWN | SOLID | DANGER,
            ticks: 0,
        }
    }
    pub const fn terrain(ticks: u16) -> Self {
        Self {
            flags: KNOWN | SOLID | MINE,
            ticks,
        }
    }
    pub const fn resource(ticks: u16) -> Self {
        Self {
            flags: KNOWN | SOLID | MINE | RESOURCE,
            ticks,
        }
    }
    pub const fn resource_shape(ticks: u16, full: bool) -> Self {
        Self {
            flags: KNOWN | MINE | RESOURCE | if full { SOLID } else { 0 },
            ticks,
        }
    }
    pub const fn passive(full: bool) -> Self {
        Self {
            flags: KNOWN | if full { SOLID } else { SUPPORT_GUARD },
            ticks: 0,
        }
    }
    pub fn is_air(self) -> bool {
        self.flags & AIR != 0
    }
    pub fn safe_neighbor(self) -> bool {
        self.flags & KNOWN != 0 && self.flags & DANGER == 0
    }
    pub fn is_resource(self) -> bool {
        self.flags & RESOURCE != 0
    }
    pub fn is_support(self) -> bool {
        self.flags & (KNOWN | SOLID | DANGER) == KNOWN | SOLID
    }
}
const DIRS: [Node; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];
fn add(a: Node, b: Node) -> Node {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[derive(Debug)]
pub struct Grid {
    origin: Node,
    shape: [usize; 3],
    cells: Vec<Cell>,
    distance: Vec<u16>,
    prepared: usize,
    resources: usize,
}
impl Grid {
    pub fn new(origin: Node, shape: [usize; 3]) -> Result<Self, String> {
        let len = shape
            .iter()
            .try_fold(1usize, |n, d| n.checked_mul(*d))
            .ok_or("Terrain grid is too large")?;
        if shape.contains(&0)
            || shape[0] > 65
            || shape[2] > 65
            || shape[1] > 512
            || len > 2_000_000
            || (0..3).any(|i| origin[i].checked_add(shape[i] as i32).is_none())
        {
            return Err("Terrain grid exceeds safety bounds".into());
        }
        Ok(Self {
            origin,
            shape,
            cells: vec![Cell::unknown(); len],
            distance: vec![u16::MAX; len],
            prepared: 0,
            resources: 0,
        })
    }
    pub fn len(&self) -> usize {
        self.cells.len()
    }
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
    pub fn position(&self, i: usize) -> Node {
        let [w, _, d] = self.shape;
        [
            self.origin[0] + (i % w) as i32,
            self.origin[1] + (i / (w * d)) as i32,
            self.origin[2] + (i / w % d) as i32,
        ]
    }
    fn index(&self, p: Node) -> Option<usize> {
        let delta = [
            i64::from(p[0]) - i64::from(self.origin[0]),
            i64::from(p[1]) - i64::from(self.origin[1]),
            i64::from(p[2]) - i64::from(self.origin[2]),
        ];
        if (0..3).any(|i| delta[i] < 0 || delta[i] >= self.shape[i] as i64) {
            return None;
        }
        Some(
            delta[0] as usize
                + self.shape[0] * (delta[2] as usize + self.shape[2] * delta[1] as usize),
        )
    }
    pub fn set(&mut self, p: Node, cell: Cell) {
        if let Some(i) = self.index(p) {
            self.resources = self.resources + usize::from(cell.is_resource())
                - usize::from(self.cells[i].is_resource());
            self.cells[i] = cell;
            self.distance[i] = if cell.is_resource() { 0 } else { u16::MAX };
        }
    }
    pub fn get(&self, p: Node) -> Cell {
        self.index(p).map_or(Cell::unknown(), |i| self.cells[i])
    }
    /// Two linear distance sweeps, budgeted independently from A* expansions.
    pub fn prepare(&mut self, budget: usize) -> bool {
        let len = self.len();
        let end = self.prepared.saturating_add(budget).min(len * 2);
        while self.prepared < end {
            let forward = self.prepared < len;
            let i = if forward {
                self.prepared
            } else {
                2 * len - 1 - self.prepared
            };
            let [w, h, d] = self.shape;
            for (coordinate, size, stride) in
                [(i % w, w, 1), (i / w % d, d, w), (i / (w * d), h, w * d)]
            {
                let neighbor = if forward {
                    coordinate.checked_sub(1).map(|_| i - stride)
                } else {
                    (coordinate + 1 < size).then_some(i + stride)
                };
                if let Some(j) = neighbor {
                    self.distance[i] = self.distance[i].min(self.distance[j].saturating_add(1));
                }
            }
            self.prepared += 1;
        }
        self.prepared == 2 * len
    }
    pub fn can_mine(&self, p: Node, tunnels: bool) -> bool {
        let c = self.get(p);
        c.flags & MINE != 0
            && (tunnels || c.is_resource())
            && DIRS.iter().all(|d| {
                let n = self.get(add(p, *d));
                n.flags & KNOWN != 0 && n.flags & (DANGER | SUPPORT_GUARD) == 0
            })
    }
    pub fn safe_stand(&self, p: Node) -> bool {
        self.get(p).is_air()
            && self.get(add(p, [0, 1, 0])).is_air()
            && self.get(add(p, [0, -1, 0])).is_support()
            && self.safe_sides(p)
    }
    fn safe_sides(&self, p: Node) -> bool {
        [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]]
            .into_iter()
            .all(|d| {
                (0..2).all(|y| {
                    let c = self.get(add(add(p, d), [0, y, 0]));
                    c.safe_neighbor()
                })
            })
    }
    /// Approach settled drops only through the same supported movement checks.
    pub fn collection_step(&self, from: Node, mined: Node) -> Option<Node> {
        let to = [mined[0], from[1].min(mined[1]), mined[2]];
        self.can_walk(from, to).then_some(to)
    }
    pub fn can_walk(&self, from: Node, to: Node) -> bool {
        (from[0] - to[0]).abs() + (from[2] - to[2]).abs() == 1
            && (from[1] - to[1]).abs() <= 1
            && self.safe_stand(from)
            && self.safe_stand(to)
            && self
                .edge(from, to, false)
                .is_some_and(|(_, a)| a.len() == 1)
    }
    fn clear(&self, p: Node, tunnels: bool) -> Option<Option<Action>> {
        let c = self.get(p);
        if c.is_air() {
            Some(None)
        } else if self.can_mine(p, tunnels) {
            Some(Some(Action::Mine(p, c.is_resource())))
        } else {
            None
        }
    }
    fn edge(&self, from: Node, to: Node, tunnels: bool) -> Option<(u32, Vec<Action>)> {
        if !self.get(add(to, [0, -1, 0])).is_support() || !self.safe_sides(to) {
            return None;
        }
        let dy = to[1] - from[1];
        if dy > 0 && !self.get(add(from, [0, 2, 0])).is_air() {
            return None;
        }
        let mut actions = Vec::new();
        let mut cost = if dy == 0 { 5 } else { 8 };
        for y in (to[1]..=(from[1] + 1).max(to[1] + 1)).rev() {
            let p = [to[0], y, to[2]];
            if let Some(a) = self.clear(p, tunnels)? {
                cost += u32::from(self.get(p).ticks.max(1));
                actions.push(a);
            }
        }
        actions.push(Action::Walk(to));
        Some((cost, actions))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Mine(Node, bool),
    Walk(Node),
}

/// Preserve the origin assumed by a remaining route. Collection may not move
/// the player onto a block that a queued clearance still needs to remove.
pub fn collect_after(
    actions: &mut std::collections::VecDeque<Action>,
    to: Option<Node>,
    quota: bool,
) -> bool {
    if quota {
        actions.clear();
    }
    if let Some(to) = to {
        let walk = Action::Walk(to);
        if actions.is_empty() {
            actions.push_front(walk);
            return true;
        }
        if actions.front() == Some(&walk) {
            return true;
        }
    }
    false
}
#[derive(Clone, Debug)]
pub struct Plan {
    pub target: Node,
    pub actions: Vec<Action>,
    pub estimated_ticks: u32,
    pub expanded: usize,
}
#[derive(Debug)]
pub enum SearchResult {
    Working,
    Found(Plan),
    NoPath,
    Limited,
}
pub struct Search {
    grid: Grid,
    start: usize,
    tunnels: bool,
    limit: usize,
    expanded: usize,
    open: BinaryHeap<Reverse<(u32, u32, usize)>>,
    cost: Vec<u32>,
    parent: Vec<usize>,
    best: Option<(u32, usize, Node, Vec<Action>)>,
    done: bool,
}
impl Search {
    pub fn new(grid: Grid, start: Node, tunnels: bool, limit: usize) -> Result<Self, String> {
        if !grid.safe_stand(start) || grid.prepared != grid.len() * 2 {
            return Err("Player is not standing in prepared safe terrain".into());
        }
        let i = grid
            .index(start)
            .ok_or("Player is outside the search area")?;
        let len = grid.len();
        let mut search = Self {
            grid,
            start: i,
            tunnels,
            limit,
            expanded: 0,
            open: BinaryHeap::new(),
            cost: vec![u32::MAX; len],
            parent: vec![usize::MAX; len],
            best: None,
            done: false,
        };
        search.cost[i] = 0;
        if search.grid.resources > 0 {
            search.open.push(Reverse((search.heuristic(i), 0, i)));
        }
        Ok(search)
    }
    fn heuristic(&self, i: usize) -> u32 {
        u32::from(self.grid.distance[i].saturating_sub(2)).div_ceil(2) * 5
    }
    fn finish(&mut self) -> SearchResult {
        self.done = true;
        let Some((cost, mut i, target, last)) = self.best.take() else {
            return SearchResult::NoPath;
        };
        let mut edges = Vec::new();
        while i != self.start {
            let previous = self.parent[i];
            if previous == usize::MAX {
                return SearchResult::NoPath;
            }
            let Some((_, actions)) = self.grid.edge(
                self.grid.position(previous),
                self.grid.position(i),
                self.tunnels,
            ) else {
                return SearchResult::NoPath;
            };
            edges.push(actions);
            i = previous;
        }
        let mut actions: Vec<_> = edges.into_iter().rev().flatten().collect();
        actions.extend(last);
        let mut removed = std::collections::HashSet::new();
        let mut stand = self.grid.position(self.start);
        for a in &actions {
            match *a {
                Action::Mine(p, _) => {
                    if p[0] == stand[0] && p[2] == stand[2] {
                        return SearchResult::NoPath;
                    }
                    removed.insert(p);
                }
                Action::Walk(p) => {
                    if removed.contains(&add(p, [0, -1, 0])) {
                        return SearchResult::NoPath;
                    }
                    stand = p;
                }
            }
        }
        SearchResult::Found(Plan {
            target,
            actions,
            estimated_ticks: cost,
            expanded: self.expanded,
        })
    }
    pub fn advance(&mut self, budget: usize) -> SearchResult {
        if self.done {
            return SearchResult::NoPath;
        }
        for _ in 0..budget {
            let Some(Reverse((estimate, cost, i))) = self.open.pop() else {
                return self.finish();
            };
            if cost != self.cost[i] {
                continue;
            }
            if self.best.as_ref().is_some_and(|b| estimate >= b.0) {
                return self.finish();
            }
            if self.expanded >= self.limit {
                self.done = true;
                return SearchResult::Limited;
            }
            self.expanded += 1;
            let p = self.grid.position(i);
            for d in [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]] {
                for dy in -1..=1 {
                    let target = add(p, [d[0], dy, d[2]]);
                    if !self.grid.get(target).is_resource()
                        || !self.grid.can_mine(target, self.tunnels)
                    {
                        continue;
                    }
                    let mut terminal = Vec::new();
                    let mut total = cost;
                    let mut allowed = true;
                    // Clear the visible upper face before a low resource; never dig blind under a wall.
                    for y in (target[1]..=target[1].max(p[1] + 1)).rev() {
                        let q = [target[0], y, target[2]];
                        match self.grid.clear(q, self.tunnels) {
                            Some(Some(a)) => {
                                total =
                                    total.saturating_add(u32::from(self.grid.get(q).ticks.max(1)));
                                terminal.push(a);
                            }
                            Some(None) => {}
                            None => {
                                allowed = false;
                                break;
                            }
                        }
                    }
                    if allowed && self.best.as_ref().is_none_or(|b| total < b.0) {
                        self.best = Some((total, i, target, terminal));
                    }
                }
            }
            for d in [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]] {
                for dy in -1..=1 {
                    let q = add(p, [d[0], dy, d[2]]);
                    let Some(j) = self.grid.index(q) else {
                        continue;
                    };
                    let Some((step, _)) = self.grid.edge(p, q, self.tunnels) else {
                        continue;
                    };
                    let next = cost.saturating_add(step);
                    if next < self.cost[j] {
                        self.cost[j] = next;
                        self.parent[j] = i;
                        self.open
                            .push(Reverse((next.saturating_add(self.heuristic(j)), next, j)));
                    }
                }
            }
        }
        SearchResult::Working
    }
}
