// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Independently implemented local navigation; no network or game actions.
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, VecDeque},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
pub const MAX_WAYPOINTS: usize = 128;
pub const MAX_BREADCRUMBS: usize = 512;
pub const MAX_SCOPES: usize = 64;
pub const MAX_JSON_BYTES: usize = 4 * 1024 * 1024;
pub const WORLD_BORDER: f64 = 29_999_984.;
const FILE: &str = "navigation.json";
static SAVE_ID: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub identity: String,
    pub dimension: String,
}
impl Scope {
    /// Identity is engine-provided server address or canonical local-world path.
    /// It is JSON data only, never a filesystem component.
    pub fn new(identity: &str, dimension: &str) -> Result<Self, String> {
        if identity.is_empty() || identity.len() > 2048 || identity.chars().any(char::is_control) {
            return Err("Invalid navigation world/server identity".into());
        }
        if dimension.is_empty()
            || dimension.len() > 256
            || !dimension
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_:./-".contains(c))
        {
            return Err("Invalid navigation dimension".into());
        }
        Ok(Self {
            identity: identity.into(),
            dimension: dimension.into(),
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        Self::new(&self.identity, &self.dimension).map(|_| ())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NavigationSettings {
    pub compass_enabled: bool,
    pub waypoint_hud: bool,
    pub breadcrumbs_enabled: bool,
    pub death_markers: bool,
    pub sample_distance: f64,
    /// Larger jumps start a new trail, never a line through unobserved space.
    pub teleport_distance: f64,
}
impl Default for NavigationSettings {
    fn default() -> Self {
        Self {
            compass_enabled: false,
            waypoint_hud: false,
            breadcrumbs_enabled: false,
            death_markers: false,
            sample_distance: 2.,
            teleport_distance: 64.,
        }
    }
}
/// A death marker requires a previously observed live player in the same scope.
#[derive(Default)]
pub struct DeathTracker {
    scope: Option<Scope>,
    armed: bool,
}
impl DeathTracker {
    pub fn observe(&mut self, enabled: bool, scope: Option<&Scope>, alive: bool) -> bool {
        if !enabled || scope.is_none() {
            *self = Self::default();
            return false;
        }
        if self.scope.as_ref() != scope {
            self.scope = scope.cloned();
            self.armed = alive;
            return false;
        }
        let died = self.armed && !alive;
        self.armed = alive;
        died
    }
}

#[cfg(test)]
mod death_tests {
    use super::*;
    #[test]
    fn death_is_emitted_once_after_life_in_the_same_scope() {
        let a = Scope::new("local:a", "minecraft:overworld").unwrap();
        let mut t = DeathTracker::default();
        assert!(!t.observe(true, Some(&a), false));
        assert!(!t.observe(true, Some(&a), true));
        assert!(t.observe(true, Some(&a), false));
        assert!(!t.observe(true, Some(&a), false));
        assert!(!t.observe(true, Some(&a), true));
        assert!(t.observe(true, Some(&a), false));
    }
    #[test]
    fn disconnect_disable_and_world_change_do_not_create_deaths() {
        let a = Scope::new("server:a", "minecraft:overworld").unwrap();
        let b = Scope::new("server:a", "minecraft:the_nether").unwrap();
        let mut t = DeathTracker::default();
        t.observe(true, Some(&a), true);
        t.observe(true, None, false);
        assert!(!t.observe(true, Some(&a), false));
        t.observe(true, Some(&a), true);
        assert!(!t.observe(true, Some(&b), false));
        t.observe(true, Some(&a), true);
        t.observe(false, Some(&a), true);
        assert!(!t.observe(true, Some(&a), false));
    }
}
impl NavigationSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !self.sample_distance.is_finite()
            || !(0.5..=16.).contains(&self.sample_distance)
            || !self.teleport_distance.is_finite()
            || !(16.0..=128.0).contains(&self.teleport_distance)
            || self.sample_distance >= self.teleport_distance
        {
            return Err("Navigation sample distance must be 0.5-16; jump distance 16-128 and greater than sample distance".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Waypoint {
    pub id: u64,
    pub name: String,
    pub position: [f64; 3],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeData {
    pub scope: Scope,
    pub waypoints: Vec<Waypoint>,
    pub selected: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationData {
    pub version: u32,
    pub next_id: u64,
    pub scopes: Vec<ScopeData>,
}
impl Default for NavigationData {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            scopes: Vec::new(),
        }
    }
}
fn position_valid(p: [f64; 3]) -> Result<(), String> {
    if p.iter().any(|x| !x.is_finite() || x.abs() > WORLD_BORDER) {
        return Err("Waypoint coordinates must be finite and inside the world border".into());
    }
    Ok(())
}
pub fn sanitize_name(name: &str) -> String {
    let mut out = String::new();
    let mut skip_format = false;
    for c in name.chars() {
        if skip_format {
            skip_format = false;
            continue;
        }
        if c == '§' {
            skip_format = true;
            continue;
        }
        if c.is_control() || "/\\<>:\"|?*".contains(c) {
            continue;
        }
        if out.chars().count() == 48 {
            break;
        }
        out.push(c);
    }
    let out = out.trim().to_owned();
    if out.is_empty() {
        "Waypoint".into()
    } else {
        out
    }
}
impl NavigationData {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.next_id == 0 || self.scopes.len() > MAX_SCOPES {
            return Err("Unsupported or oversized navigation data".into());
        }
        let mut scopes = HashSet::new();
        let mut ids = HashSet::new();
        for entry in &self.scopes {
            entry.scope.validate()?;
            if !scopes.insert(&entry.scope) || entry.waypoints.len() > MAX_WAYPOINTS {
                return Err("Duplicate or oversized waypoint scope".into());
            }
            for w in &entry.waypoints {
                position_valid(w.position)?;
                if w.id == 0 || w.id >= self.next_id || !ids.insert(w.id) {
                    return Err("Duplicate or invalid waypoint id".into());
                }
                if w.name != sanitize_name(&w.name) {
                    return Err("Invalid waypoint name".into());
                }
            }
            if entry
                .selected
                .is_some_and(|id| !entry.waypoints.iter().any(|w| w.id == id))
            {
                return Err("Selected waypoint does not exist in its scope".into());
            }
        }
        Ok(())
    }
    pub fn add(&mut self, scope: &Scope, name: &str, position: [f64; 3]) -> Result<u64, String> {
        self.validate()?;
        scope.validate()?;
        position_valid(position)?;
        let id = self.next_id;
        let next_id = id.checked_add(1).ok_or("Waypoint id limit reached")?;
        let index = if let Some(i) = self.scopes.iter().position(|s| &s.scope == scope) {
            i
        } else {
            if self.scopes.len() == MAX_SCOPES {
                return Err("Navigation world/server limit reached".into());
            }
            self.scopes.push(ScopeData {
                scope: scope.clone(),
                waypoints: Vec::new(),
                selected: None,
            });
            self.scopes.len() - 1
        };
        let entry = &mut self.scopes[index];
        if entry.waypoints.len() == MAX_WAYPOINTS {
            return Err("Waypoint limit reached for this world and dimension".into());
        }
        entry.waypoints.push(Waypoint {
            id,
            name: sanitize_name(name),
            position,
        });
        self.next_id = next_id;
        Ok(id)
    }
    pub fn waypoints(&self, scope: &Scope) -> &[Waypoint] {
        self.scopes
            .iter()
            .find(|s| &s.scope == scope)
            .map_or(&[], |s| &s.waypoints)
    }
    pub fn selected(&self, scope: &Scope) -> Option<&Waypoint> {
        let entry = self.scopes.iter().find(|s| &s.scope == scope)?;
        entry
            .waypoints
            .iter()
            .find(|w| Some(w.id) == entry.selected)
    }
    pub fn select(&mut self, scope: &Scope, id: Option<u64>) -> Result<(), String> {
        let entry = self
            .scopes
            .iter_mut()
            .find(|s| &s.scope == scope)
            .ok_or("No waypoints for this scope")?;
        if id.is_some_and(|id| !entry.waypoints.iter().any(|w| w.id == id)) {
            return Err("Waypoint does not exist in this scope".into());
        }
        entry.selected = id;
        Ok(())
    }
    pub fn remove(&mut self, scope: &Scope, id: u64) -> Result<(), String> {
        let entry = self
            .scopes
            .iter_mut()
            .find(|s| &s.scope == scope)
            .ok_or("No waypoints for this scope")?;
        let index = entry
            .waypoints
            .iter()
            .position(|w| w.id == id)
            .ok_or("Waypoint does not exist in this scope")?;
        entry.waypoints.remove(index);
        if entry.selected == Some(id) {
            entry.selected = None;
        }
        Ok(())
    }
    pub fn coordinates(&self, scope: &Scope, id: u64) -> Result<String, String> {
        let w = self
            .waypoints(scope)
            .iter()
            .find(|w| w.id == id)
            .ok_or("Waypoint does not exist in this scope")?;
        Ok(format!(
            "{:.2} {:.2} {:.2}",
            w.position[0], w.position[1], w.position[2]
        ))
    }
    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        if json.len() > MAX_JSON_BYTES {
            return Err("Navigation data is too large".into());
        }
        Ok(json)
    }
    pub fn from_json(text: &str) -> Result<Self, String> {
        if text.len() > MAX_JSON_BYTES {
            return Err("Navigation data is too large".into());
        }
        let data: Self =
            serde_json::from_str(text).map_err(|e| format!("Invalid navigation data: {e}"))?;
        data.validate()?;
        Ok(data)
    }
    /// Fixed filenames only; validation and durable temp write precede atomic
    /// replacement. Errors preserve the destination, with no delete-first gap.
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        let json = self.to_json()?;
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let temporary = dir.join(format!(
            "navigation-{}-{}.tmp",
            std::process::id(),
            SAVE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| -> std::io::Result<()> {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            file.write_all(json.as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, dir.join(FILE))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(|e| format!("Could not save navigation: {e}"))
    }
    pub fn load(dir: &Path) -> Result<Self, String> {
        let file = match fs::File::open(dir.join(FILE)) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(format!("Could not read navigation: {e}")),
        };
        let mut json = String::new();
        file.take(MAX_JSON_BYTES as u64 + 1)
            .read_to_string(&mut json)
            .map_err(|e| e.to_string())?;
        Self::from_json(&json)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Direction {
    pub distance: f64,
    pub bearing: f64,
    pub relative: f64,
}
/// Minecraft yaw: south=0, west=90, north=180, east=270.
pub fn compass(yaw: f64) -> Result<&'static str, String> {
    if !yaw.is_finite() {
        return Err("Invalid compass heading".into());
    }
    Ok(["S", "SW", "W", "NW", "N", "NE", "E", "SE"]
        [(((yaw.rem_euclid(360.) + 22.5) / 45.).floor() as usize) % 8])
}
pub fn direction(from: [f64; 3], to: [f64; 3], yaw: f64) -> Result<Direction, String> {
    position_valid(from)?;
    position_valid(to)?;
    compass(yaw)?;
    let dx = to[0] - from[0];
    let dz = to[2] - from[2];
    let bearing = if dx == 0. && dz == 0. {
        yaw.rem_euclid(360.)
    } else {
        (-dx).atan2(dz).to_degrees().rem_euclid(360.)
    };
    let distance = from
        .iter()
        .zip(to)
        .map(|(a, b)| (b - a).powi(2))
        .sum::<f64>()
        .sqrt();
    Ok(Direction {
        distance,
        bearing,
        relative: (bearing - yaw + 180.).rem_euclid(360.) - 180.,
    })
}
/// Ephemeral observed body positions, never persisted or extrapolated.
#[derive(Default)]
pub struct BreadcrumbTrail {
    scope: Option<Scope>,
    points: VecDeque<[f64; 3]>,
    previous_observed: Option<[f64; 3]>,
}
impl BreadcrumbTrail {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn observe(
        &mut self,
        scope: &Scope,
        position: [f64; 3],
        loaded: bool,
        settings: &NavigationSettings,
    ) -> Result<(), String> {
        settings.validate()?;
        scope.validate()?;
        if position_valid(position).is_err() {
            self.reset();
            return Err("Invalid breadcrumb position".into());
        }
        if !settings.breadcrumbs_enabled || !loaded {
            self.reset();
            return Ok(());
        }
        if self.scope.as_ref() != Some(scope) {
            self.reset();
            self.scope = Some(scope.clone());
        }
        if self
            .previous_observed
            .is_some_and(|old| distance_squared(old, position) > settings.teleport_distance.powi(2))
        {
            self.points.clear();
        }
        self.previous_observed = Some(position);
        if self
            .points
            .back()
            .is_none_or(|old| distance_squared(*old, position) >= settings.sample_distance.powi(2))
        {
            self.points.push_back(position);
            if self.points.len() > MAX_BREADCRUMBS {
                self.points.pop_front();
            }
        }
        Ok(())
    }
    pub fn points(&self) -> &VecDeque<[f64; 3]> {
        &self.points
    }
    pub fn segments(&self) -> Vec<[[f64; 3]; 2]> {
        self.points
            .iter()
            .zip(self.points.iter().skip(1))
            .map(|(a, b)| [*a, *b])
            .collect()
    }
}
fn distance_squared(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| (b - a).powi(2)).sum()
}
