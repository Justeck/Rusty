// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Pure safety contracts shared by native construction and independent tests.
use super::rusty_mining::{Grid, Node};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;
/// Only a received hit for the captured local player and world can stop a run.
/// This latch is independent of health deltas, absorption and later healing.
pub struct DamageWatch<S> {
    scope: S,
    entity: i32,
    received: bool,
}
impl<S: PartialEq> DamageWatch<S> {
    pub fn new(scope: S, entity: i32) -> Self {
        Self {
            scope,
            entity,
            received: false,
        }
    }
    pub fn receive(&mut self, scope: &S, local: Option<i32>, hurt: i32) -> bool {
        if scope != &self.scope || local != Some(self.entity) || hurt != self.entity {
            return false;
        }
        self.received = true;
        true
    }
    pub fn received(&self) -> bool {
        self.received
    }
}

/// DryLevel collects its final writes from an unordered HashMap. Compare the
/// exact bounded position/state identities, never that map's iteration order.
/// Duplicate positions are invalid even if their states agree in both inputs.
pub fn same_writes(expected: &[(Node, u32)], actual: &[(Node, u32)]) -> bool {
    fn unique(writes: &[(Node, u32)]) -> bool {
        writes
            .iter()
            .enumerate()
            .all(|(i, (at, _))| !writes[..i].iter().any(|(other, _)| at == other))
    }
    !expected.is_empty()
        && expected.len() <= 16
        && expected.len() == actual.len()
        && unique(expected)
        && unique(actual)
        && expected.iter().all(|entry| actual.contains(entry))
}

/// Origin-only reactivation shares immutable data, even when M/P cannot clone.
pub struct ResolvedData<M, P> {
    pub model: Arc<M>,
    pub palette: Arc<Vec<P>>,
}
impl<M, P> ResolvedData<M, P> {
    pub fn shared(&self) -> Self {
        Self {
            model: Arc::clone(&self.model),
            palette: Arc::clone(&self.palette),
        }
    }
}

#[derive(Default, Clone, Debug, serde::Serialize)]
pub struct OutcomeSummary {
    pub initially_matching: Option<usize>,
    pub finally_matching: usize,
    pub completed: usize,
    pub occupied: usize,
    pub remaining_missing: usize,
    pub unsupported_inaccessible: usize,
    pub remaining_manual: usize,
    pub unknown: usize,
    pub ignored_block_entities: u32,
    pub ignored_entities: u32,
    pub survey_complete: bool,
}
#[derive(Default, Clone)]
pub struct OutcomeCounts {
    pub matching: usize,
    pub missing: usize,
    pub occupied: usize,
    pub unknown: usize,
    pub manual: usize,
    pub unsupported: usize,
    pub completed: usize,
}
impl OutcomeCounts {
    pub fn observe(&mut self, kind: u8, manual: bool, unsupported: bool, completed: bool) {
        match kind {
            0 => self.unknown += 1,
            1 => {
                self.matching += 1;
                self.completed += usize::from(completed);
            }
            2 => {
                self.missing += 1;
                self.manual += usize::from(manual);
                self.unsupported += usize::from(unsupported);
            }
            _ => self.occupied += 1,
        }
    }
}
impl OutcomeSummary {
    pub fn observed(
        counts: &OutcomeCounts,
        initially_matching: Option<usize>,
        ignored_block_entities: u32,
        ignored_entities: u32,
        scan_complete: bool,
    ) -> Self {
        Self {
            initially_matching,
            finally_matching: counts.matching,
            completed: counts.completed,
            occupied: counts.occupied,
            remaining_missing: counts.missing,
            unsupported_inaccessible: counts.unsupported,
            remaining_manual: counts.manual,
            unknown: counts.unknown,
            ignored_block_entities,
            ignored_entities,
            survey_complete: scan_complete && counts.unknown == 0,
        }
    }
}

/// Permission to classify one exact owned prediction as a safe SIDE neighbor
/// while frozen awaiting its real receipt. Never applies to feet/head/support.
pub struct PendingNeighbor {
    stand: Node,
    at: Node,
    sequence: i32,
    state: u32,
    after: super::rusty_mining::Cell,
}
impl PendingNeighbor {
    pub fn capture(
        stand: Node,
        at: Node,
        sequence: i32,
        state: u32,
        before: super::rusty_mining::Cell,
        after: super::rusty_mining::Cell,
    ) -> Option<Self> {
        let side = (i64::from(at[0]) - i64::from(stand[0])).abs()
            + (i64::from(at[2]) - i64::from(stand[2])).abs()
            == 1;
        (side
            && (0..=1).contains(&(i64::from(at[1]) - i64::from(stand[1])))
            && before.safe_neighbor()
            && after.safe_neighbor())
        .then_some(Self {
            stand,
            at,
            sequence,
            state,
            after,
        })
    }
    pub fn guard_cell(
        &self,
        stand: Node,
        at: Node,
        sequence: Option<i32>,
        actual: u32,
        received_chunk: bool,
    ) -> Option<super::rusty_mining::Cell> {
        (stand == self.stand
            && at == self.at
            && sequence == Some(self.sequence)
            && actual == self.state
            && received_chunk)
            .then_some(self.after)
    }
}

/// Vanilla REQUEST_STATS is processed in the same ordered server packet queue
/// as ContainerClick, and only that request emits AwardStats. Every request,
/// including the stock Stats screen's, must be counted for FIFO correlation.
#[derive(Default, Debug)]
pub struct StatsFence {
    sent: u64,
    received: u64,
    broken: bool,
}
impl StatsFence {
    pub fn request(&mut self) -> Result<u64, String> {
        if self.broken || self.sent == u64::MAX {
            return Err("Statistics response correlation is unavailable on this connection".into());
        }
        self.sent += 1;
        Ok(self.sent)
    }
    pub fn response(&mut self) {
        if self.received >= self.sent {
            self.broken = true;
        } else {
            self.received += 1;
        }
    }
    pub fn confirmed(&self, ordinal: u64) -> bool {
        !self.broken && ordinal > 0 && ordinal <= self.received
    }
    pub fn last_request(&self) -> u64 {
        self.sent
    }
    pub fn reliable(&self) -> bool {
        !self.broken
    }
    pub fn snapshot(&self) -> (u64, u64, bool) {
        (self.sent, self.received, !self.broken)
    }
}
pub fn approach_matches(
    name: &str,
    properties: &BTreeMap<String, String>,
    stand: Node,
    target: Node,
) -> bool {
    if !(name.ends_with("_stairs") || name.ends_with("_door") || name.ends_with("_bed")) {
        return true;
    }
    let dx = i64::from(target[0]) - i64::from(stand[0]);
    let dz = i64::from(target[2]) - i64::from(stand[2]);
    match properties.get("facing").map(String::as_str) {
        Some("north") => dz < 0 && dz.abs() > dx.abs(),
        Some("south") => dz > 0 && dz.abs() > dx.abs(),
        Some("east") => dx > 0 && dx.abs() > dz.abs(),
        Some("west") => dx < 0 && dx.abs() > dz.abs(),
        _ => true,
    }
}

pub fn validate_writes(
    plan: &BTreeMap<Node, u32>,
    target: Node,
    writes: &[(Node, u32)],
) -> Result<(), String> {
    if writes.is_empty() || writes.len() > 16 || !writes.iter().any(|(at, _)| *at == target) {
        return Err("Ordinary placement did not write its intended target".into());
    }
    let mut seen = BTreeSet::new();
    for &(at, state) in writes {
        if !seen.insert(at) || plan.get(&at) != Some(&state) {
            return Err(
                "Ordinary placement would write an unplanned or incorrect block state".into(),
            );
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct SwapReceipt {
    pub source: usize,
    pub target: usize,
    seen: [bool; 2],
    rejected: bool,
}
impl SwapReceipt {
    pub fn new(source: usize, target: usize) -> Result<Self, String> {
        if !(9..36).contains(&source) || target >= 9 {
            return Err("Invalid ordinary inventory swap".into());
        }
        Ok(Self {
            source,
            target,
            seen: [false; 2],
            rejected: false,
        })
    }
    pub fn observe(&mut self, slot: usize, exact: bool) -> bool {
        let index = if slot == self.source {
            Some(0)
        } else if slot == self.target {
            Some(1)
        } else {
            None
        };
        if let Some(index) = index {
            self.seen[index] = true;
            self.rejected |= !exact;
        }
        self.complete()
    }
    pub fn complete(&self) -> bool {
        self.seen == [true; 2] && !self.rejected
    }
    pub fn rejected(&self) -> bool {
        self.rejected
    }
}

pub fn material_cost(properties: &BTreeMap<String, String>) -> u32 {
    if properties.get("half").is_some_and(|v| v == "upper")
        || properties.get("part").is_some_and(|v| v == "head")
    {
        0
    } else if properties.get("type").is_some_and(|v| v == "double") {
        2
    } else {
        ["layers", "candles", "eggs", "pickles"]
            .into_iter()
            .find_map(|k| properties.get(k).and_then(|n| n.parse::<u32>().ok()))
            .unwrap_or(1)
    }
}

/// Only a known monotone ordinary placement stage may precede the final state.
/// The final survey still requires the exact desired state, never this predicate.
pub fn safe_progress(
    name: &str,
    desired: &BTreeMap<String, String>,
    previous_name: &str,
    previous: &BTreeMap<String, String>,
    written_name: &str,
    written: &BTreeMap<String, String>,
) -> bool {
    if name != written_name {
        return false;
    }
    let key = if name.ends_with("_slab") && desired.get("type").is_some_and(|v| v == "double") {
        "type"
    } else if name == "minecraft:snow" {
        "layers"
    } else if name == "minecraft:sea_pickle" {
        "pickles"
    } else if name == "minecraft:turtle_egg" {
        "eggs"
    } else if name.ends_with("_candle") || name == "minecraft:candle" {
        "candles"
    } else {
        return false;
    };
    if desired.len() != written.len()
        || desired
            .iter()
            .any(|(k, v)| k != key && written.get(k) != Some(v))
    {
        return false;
    }
    let air = matches!(
        previous_name,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    );
    if !air
        && (previous_name != name
            || previous.len() != desired.len()
            || desired
                .iter()
                .any(|(k, v)| k != key && previous.get(k) != Some(v)))
    {
        return false;
    }
    if key == "type" {
        return air
            && written
                .get(key)
                .is_some_and(|v| v == "bottom" || v == "top");
    }
    let count = |p: &BTreeMap<String, String>| p.get(key).and_then(|v| v.parse::<u32>().ok());
    let Some(goal) = count(desired) else {
        return false;
    };
    let Some(next) = count(written) else {
        return false;
    };
    let old = if air {
        0
    } else {
        count(previous).unwrap_or(u32::MAX)
    };
    next < goal && next > 0 && old.checked_add(1) == Some(next)
}
pub fn add_material(
    totals: &mut BTreeMap<String, u32>,
    item: &str,
    properties: &BTreeMap<String, String>,
    count: u32,
) {
    let cost = material_cost(properties) * count;
    if cost > 0 {
        *totals.entry(item.to_owned()).or_default() += cost;
    }
}
pub fn stack_breakdown(count: u32, max_stack: u32) -> Option<(u32, u32)> {
    (max_stack > 0).then(|| (count / max_stack, count % max_stack))
}

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: Node,
    pub size: [u32; 3],
}
impl Bounds {
    pub fn new(a: Node, b: Node) -> Result<Self, String> {
        let min = std::array::from_fn(|i| a[i].min(b[i]));
        let mut size = [0; 3];
        for i in 0..3 {
            let d = (i64::from(a[i]) - i64::from(b[i])).unsigned_abs() + 1;
            if d > 256 {
                return Err("Selection exceeds 256 blocks on an axis".into());
            }
            size[i] = d as u32;
        }
        let out = Self { min, size };
        if out.len() > 1_048_576 {
            return Err("Selection exceeds 1,048,576 cells".into());
        }
        Ok(out)
    }
    pub fn len(self) -> usize {
        self.size.iter().map(|x| *x as usize).product()
    }
    pub fn position(self, i: usize) -> Node {
        let [w, _, d] = self.size.map(|v| v as usize);
        [
            self.min[0] + (i % w) as i32,
            self.min[1] + (i / (w * d)) as i32,
            self.min[2] + (i / w % d) as i32,
        ]
    }
}

#[derive(Default, Debug)]
pub struct Generation {
    counter: u64,
    pending: bool,
}
impl Generation {
    pub fn begin(&mut self) -> u64 {
        self.counter = self.counter.wrapping_add(1);
        self.pending = true;
        self.counter
    }
    pub fn cancel(&mut self) {
        self.pending = false;
        self.counter = self.counter.wrapping_add(1);
    }
    pub fn accepts(&self, token: u64) -> bool {
        self.pending && token == self.counter
    }
}

/// Bounded breadth-first search using ONLY the established no-mining walk edge.
pub fn walk_route(
    grid: &Grid,
    start: Node,
    goal: impl Fn(Node) -> bool,
    budget: usize,
) -> Option<VecDeque<Node>> {
    if !grid.safe_stand(start) {
        return None;
    }
    let mut from = BTreeMap::from([(start, start)]);
    let mut open = VecDeque::from([start]);
    for _ in 0..budget {
        let p = open.pop_front()?;
        if goal(p) {
            let mut route = VecDeque::new();
            let mut q = p;
            while q != start {
                route.push_front(q);
                q = from[&q];
            }
            return Some(route);
        }
        for [dx, dz] in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
            for dy in [0, 1, -1] {
                let q = [p[0] + dx, p[1] + dy, p[2] + dz];
                if !from.contains_key(&q) && grid.can_walk(p, q) {
                    from.insert(q, p);
                    open.push_back(q);
                }
            }
        }
    }
    None
}

/// Independent of construction STATE: ordinary actions may mutate the world while it is borrowed.
#[derive(Default)]
pub struct SurveyWatch {
    owner: usize,
    columns: BTreeSet<(i32, i32)>,
    revision: u64,
}
impl SurveyWatch {
    pub fn watch(&mut self, owner: usize, columns: BTreeSet<(i32, i32)>) {
        if self.owner != owner || self.columns != columns {
            self.owner = owner;
            self.columns = columns;
            self.revision = self.revision.wrapping_add(1);
        }
    }
    pub fn changed(&mut self, owner: usize, column: Option<(i32, i32)>) {
        if owner == self.owner && column.is_none_or(|c| self.columns.contains(&c)) {
            self.revision = self.revision.wrapping_add(1);
        }
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
}
thread_local! { static SURVEY_WATCH: std::cell::RefCell<SurveyWatch> = std::cell::RefCell::new(SurveyWatch::default()); }
pub fn survey_watch(owner: usize, columns: BTreeSet<(i32, i32)>) {
    SURVEY_WATCH.with_borrow_mut(|w| w.watch(owner, columns));
}
pub fn survey_changed(owner: usize, column: Option<(i32, i32)>) {
    SURVEY_WATCH.with_borrow_mut(|w| w.changed(owner, column));
}
pub fn survey_revision() -> u64 {
    SURVEY_WATCH.with_borrow(SurveyWatch::revision)
}

#[derive(Default, Clone)]
pub struct SurveyPass {
    revision: Option<u64>,
    restarts: u8,
    pub exhausted: bool,
}
impl SurveyPass {
    /// True means discard all earlier counts before reading the next bounded batch.
    pub fn begin(&mut self, now: u64) -> bool {
        if self.revision == Some(now) {
            return false;
        }
        if self.revision.is_some() {
            self.restarts = self.restarts.saturating_add(1);
            self.exhausted = self.restarts > 8;
        }
        self.revision = Some(now);
        true
    }
    pub fn coherent(&self, now: u64) -> bool {
        !self.exhausted && self.revision == Some(now)
    }
}

#[cfg(test)]
mod coherent_survey_tests {
    use super::*;
    #[test]
    fn already_scanned_mutation_restarts_each_kind_then_stable_pass_completes() {
        for _kind in [
            "block",
            "prediction settlement",
            "resident unlit replacement",
            "unload",
            "range eviction",
        ] {
            let mut watch = SurveyWatch::default();
            watch.watch(10, [(0, 0), (1, 0)].into());
            let mut pass = SurveyPass::default();
            let mut counts = OutcomeCounts::default();
            assert!(pass.begin(watch.revision()));
            for _ in 0..2048 {
                counts.observe(1, false, false, false);
            }
            watch.changed(10, Some((0, 0)));
            assert!(!pass.coherent(watch.revision()));
            assert!(pass.begin(watch.revision()));
            counts = OutcomeCounts::default();
            counts.observe(2, false, false, false);
            for _ in 1..4096 {
                counts.observe(1, false, false, false);
            }
            assert_eq!(counts.matching, 4095);
            assert!(pass.coherent(watch.revision()));
            // The next stable read may truthfully complete after the missing cell is received.
            watch.changed(10, Some((0, 0)));
            assert!(pass.begin(watch.revision()));
            assert!(pass.coherent(watch.revision()));
        }
    }
    #[test]
    fn thread_local_watcher_is_independent_and_scoped() {
        survey_watch(100, [(2, 3)].into());
        let before = survey_revision();
        survey_changed(99, Some((2, 3)));
        assert_eq!(before, survey_revision());
        survey_changed(100, Some((2, 3)));
        assert_ne!(before, survey_revision());
    }
    #[test]
    fn foreign_world_and_far_activity_do_not_starve_but_target_churn_is_bounded() {
        let mut watch = SurveyWatch::default();
        watch.watch(10, [(0, 0)].into());
        let revision = watch.revision();
        watch.changed(20, Some((0, 0)));
        watch.changed(10, Some((200, 200)));
        assert_eq!(revision, watch.revision());
        let mut pass = SurveyPass::default();
        pass.begin(revision);
        for _ in 0..9 {
            watch.changed(10, None);
            pass.begin(watch.revision());
        }
        assert!(pass.exhausted);
        assert!(!pass.coherent(watch.revision()));
    }
}
