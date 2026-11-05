// Rusty native configuration and selection. Modified 2026 Justeck, Apache-2.0; see NOTICE.rs.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

pub const ID: &str = "rusty";
pub const VERSION: &str = "1.2.7";
pub const TITLE: &str = "Rusty";
pub const DEFAULT_KEY: &str = "key.keyboard.x";
pub const MAX_PROFILE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationMode {
    #[default]
    Toggle,
    Hold,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BlockRule {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub color: u32,
    pub radius: u32,
    pub min_y: i32,
    pub max_y: i32,
    pub states: BTreeMap<String, String>,
    pub tracers: bool,
    pub outline: bool,
    pub group: String,
}
impl Default for BlockRule {
    fn default() -> Self {
        Self::new("minecraft:diamond_ore")
    }
}
impl BlockRule {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.into(),
            label: String::new(),
            enabled: true,
            color: 0xff55dede,
            radius: 64,
            min_y: -64,
            max_y: 319,
            states: BTreeMap::new(),
            tracers: true,
            outline: true,
            group: "Custom".into(),
        }
    }
    pub fn contains(&self, pos: [i32; 3], origin: [f64; 3]) -> bool {
        let dx = f64::from(pos[0]) + 0.5 - origin[0];
        let dz = f64::from(pos[2]) + 0.5 - origin[2];
        self.enabled
            && pos[1] >= self.min_y
            && pos[1] <= self.max_y
            && dx * dx + dz * dz <= f64::from(self.radius).powi(2)
    }
    pub fn display_name(&self) -> String {
        if self.label.trim().is_empty() {
            friendly_name(&self.id)
        } else {
            self.label.clone()
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub schema: u32,
    pub client: super::rusty_catalog::ClientPreferences,
    pub navigation: super::rusty_navigation::NavigationSettings,
    pub utilities: super::rusty_utility::UtilitySettings,
    pub awareness: super::rusty_awareness::AwarenessSettings,
    pub assistance: super::rusty_assistance::AssistanceSettings,
    pub build: super::rusty_build_plan::BuildSettings,
    pub activation: ActivationMode,
    pub context_outlines: bool,
    pub context_radius: u32,
    pub context_opacity: u8,
    pub context_border_width: f32,
    pub block_outlines: bool,
    pub tracers: bool,
    pub tracer_limit: u32,
    pub line_width: f32,
    pub finder_enabled: bool,
    pub auto_scan: bool,
    pub scan_radius: u32,
    pub scan_interval: u32,
    pub max_targets: u32,
    pub hud: bool,
    pub enabled_mods: bool,
    pub hud_counts: bool,
    pub fullbright: bool,
    pub no_fog: bool,
    pub clear_weather: bool,
    pub entity_esp: bool,
    pub esp_players: bool,
    pub esp_mobs: bool,
    pub esp_items: bool,
    pub zoom_factor: f32,
    pub coords_hud: bool,
    pub auto_sprint: bool,
    pub no_hurt_camera: bool,
    pub freecam_noclip: bool,
    pub freecam_speed: f64,
    pub rules: Vec<BlockRule>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            schema: 1,
            client: Default::default(),
            navigation: Default::default(),
            utilities: Default::default(),
            awareness: Default::default(),
            assistance: Default::default(),
            build: Default::default(),
            activation: ActivationMode::Toggle,
            context_outlines: true,
            context_radius: 18,
            context_opacity: 255,
            context_border_width: 6.698_73,
            block_outlines: true,
            tracers: true,
            tracer_limit: 12,
            line_width: 1.2,
            finder_enabled: false,
            auto_scan: true,
            scan_radius: 64,
            scan_interval: 5,
            max_targets: 1024,
            hud: true,
            enabled_mods: false,
            hud_counts: false,
            fullbright: false,
            no_fog: false,
            clear_weather: false,
            entity_esp: false,
            esp_players: true,
            esp_mobs: false,
            esp_items: false,
            zoom_factor: 4.0,
            coords_hud: false,
            auto_sprint: false,
            no_hurt_camera: false,
            freecam_noclip: true,
            freecam_speed: 1.0,
            rules: default_rules(),
        }
    }
}
impl Config {
    /// Restore one stored field without replacing unrelated preferences.
    pub fn restore_default_path(&mut self, path: &str) -> Result<(), String> {
        if !path.starts_with('/') || path.len() > 256 {
            return Err("Unsupported default setting".into());
        }
        let defaults = serde_json::to_value(Self::default()).map_err(|e| e.to_string())?;
        let replacement = defaults
            .pointer(path)
            .ok_or("Unknown default setting")?
            .clone();
        let mut current = serde_json::to_value(&*self).map_err(|e| e.to_string())?;
        *current.pointer_mut(path).ok_or("Unknown setting")? = replacement;
        let next: Self = serde_json::from_value(current).map_err(|e| e.to_string())?;
        *self = next.validated()?;
        Ok(())
    }
    /// Visible area of each ordinary block face. The zero endpoint still has
    /// a thin textured edge, while the full endpoint uses native terrain.
    pub fn texture_visibility(&self) -> f32 {
        let w = (self.context_border_width / 100.).clamp(0.001, 0.5);
        if w <= 0.001_001 {
            0.
        } else {
            (1. - (1. - 2. * w).powi(2)) * 100.
        }
    }
    pub fn set_texture_visibility(&mut self, percent: f32) -> Result<(), String> {
        if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
            return Err("Block texture visibility must be between 0 and 100%".into());
        }
        self.context_border_width = ((1. - (1. - percent / 100.).sqrt()) * 50.).max(0.1);
        Ok(())
    }
    pub fn normal_terrain(&self) -> bool {
        self.context_outlines && self.context_border_width >= 50.
    }
    /// Four disjoint strips around the original face; the open center reveals
    /// selected blocks behind it. Coordinates interpolate both geometry and UV.
    pub fn terrain_frame_rects(&self) -> [[f32; 4]; 4] {
        let w = (self.context_border_width / 100.).clamp(0.001, 0.5);
        [
            [0., 0., 1., w],
            [0., 1. - w, 1., 1.],
            [0., w, w, 1. - w],
            [1. - w, w, 1., 1. - w],
        ]
    }
    pub fn validated(mut self) -> Result<Self, String> {
        self.client.validate()?;
        self.navigation.validate()?;
        self.utilities.validate()?;
        self.awareness.validate()?;
        self.assistance.validate()?;
        self.build.validate()?;
        if self.schema != 1 {
            return Err("Unsupported profile schema (expected 1)".into());
        }
        if self.rules.len() > 256 {
            return Err("A profile can contain at most 256 block rules".into());
        }
        if !(1..=48).contains(&self.context_radius)
            || !(1..=128).contains(&self.scan_radius)
            || !(1..=60).contains(&self.scan_interval)
            || !(1..=4096).contains(&self.max_targets)
            || self.tracer_limit > 12
        {
            return Err(
                "Search radius, scan interval or rendering limit is outside its supported range"
                    .into(),
            );
        }
        if !self.line_width.is_finite()
            || !(0.5..=4.0).contains(&self.line_width)
            || !self.context_border_width.is_finite()
            || !(0.1..=50.0).contains(&self.context_border_width)
            || !self.zoom_factor.is_finite()
            || !(1.0..=16.0).contains(&self.zoom_factor)
            || !self.freecam_speed.is_finite()
            || !(0.05..=8.0).contains(&self.freecam_speed)
        {
            return Err("Invalid line width, zoom factor or camera speed".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for rule in &mut self.rules {
            rule.id =
                normalize_id(&rule.id).ok_or_else(|| format!("Invalid block ID: {}", rule.id))?;
            if !ids.insert(rule.id.clone()) {
                return Err(format!(
                    "Duplicate block ID {}; use one rule per block ID",
                    rule.id
                ));
            }
            if rule.min_y > rule.max_y
                || rule.min_y < -2048
                || rule.max_y > 2047
                || !(1..=128).contains(&rule.radius)
            {
                return Err(format!(
                    "Invalid height bounds or scan radius for {}",
                    rule.id
                ));
            }
            if rule.label.len() > 96 || rule.group.len() > 48 || rule.states.len() > 32 {
                return Err("Block label, group or property list is too long".into());
            }
            for (key, value) in &rule.states {
                if !property_token(key) || !property_token(value) {
                    return Err(format!("Invalid block property {key}={value}"));
                }
            }
        }
        Ok(self)
    }
    pub fn from_json(text: &str) -> Result<Self, String> {
        if text.len() > MAX_PROFILE_BYTES {
            return Err("Profile exceeds the 4 MiB size limit".into());
        }
        let mut config: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        // Old native tracers covered all players and mobs, independently of ESP.
        // Migrate only an existing awareness object; fresh settings select players.
        if let Some(awareness) = value.get("awareness").and_then(|a| a.as_object()) {
            if !awareness.contains_key("tracer_players") {
                config.awareness.tracer_players = true;
            }
            if !awareness.contains_key("tracer_mobs") {
                config.awareness.tracer_mobs = true;
            }
            // Missing item selection already defaults to false.
        }
        config.validated()
    }
    pub fn read_json(reader: impl std::io::Read) -> Result<Self, String> {
        use std::io::Read;
        let mut text = String::new();
        reader
            .take(MAX_PROFILE_BYTES as u64 + 1)
            .read_to_string(&mut text)
            .map_err(|error| format!("Could not read profile: {error}"))?;
        Self::from_json(&text)
    }
    /// Feature presets do not replace this player's menu organization or theme.
    /// Horizon owns key bindings separately from this configuration file.
    pub fn import_profile(&self, text: &str) -> Result<Self, String> {
        let mut imported = Self::from_json(text)?;
        imported.client = self.client.clone();
        imported.validated()
    }
    pub fn import_profile_reader(&self, reader: impl std::io::Read) -> Result<Self, String> {
        let mut imported = Self::read_json(reader)?;
        imported.client = self.client.clone();
        imported.validated()
    }
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }
    pub fn add_block(&mut self, id: &str) -> bool {
        let Some(id) = normalize_id(id) else {
            return false;
        };
        if self.rules.len() >= 256 || self.rules.iter().any(|r| r.id == id) {
            return false;
        }
        self.rules.push(BlockRule::new(&id));
        true
    }
    pub fn reset_ores(&mut self) {
        self.rules = default_rules();
    }
}
pub fn default_rules() -> Vec<BlockRule> {
    let mut rules = Vec::new();
    for (ore, color) in [
        ("coal", 0xffaaaaaa),
        ("iron", 0xffe5c5aa),
        ("copper", 0xffe49967),
        ("gold", 0xffffd64c),
        ("redstone", 0xffff5b65),
        ("emerald", 0xff53ee8d),
        ("lapis", 0xff638bff),
        ("diamond", 0xff58e6eb),
    ] {
        for prefix in ["", "deepslate_"] {
            let mut rule = BlockRule::new(&format!("minecraft:{prefix}{ore}_ore"));
            rule.color = color;
            rule.group = "Ores".into();
            rules.push(rule)
        }
    }
    for (id, color) in [
        ("nether_gold_ore", 0xffffd64c),
        ("nether_quartz_ore", 0xffeee6dc),
        ("ancient_debris", 0xffbb8e7d),
    ] {
        let mut r = BlockRule::new(&format!("minecraft:{id}"));
        r.color = color;
        r.group = "Ores".into();
        rules.push(r)
    }
    rules
}
fn property_token(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-.".contains(&b))
}
pub fn normalize_id(id: &str) -> Option<String> {
    let id = id.trim().to_ascii_lowercase();
    let id = if id.contains(':') {
        id
    } else {
        format!("minecraft:{id}")
    };
    let (namespace, path) = id.split_once(':')?;
    if namespace.is_empty()
        || path.is_empty()
        || id.len() > 192
        || !namespace
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-.".contains(&b))
        || !path
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-./".contains(&b))
    {
        return None;
    }
    Some(id)
}
pub fn friendly_name(id: &str) -> String {
    id.strip_prefix("minecraft:")
        .unwrap_or(id)
        .replace('_', " ")
}
pub fn parse_states(text: &str) -> Result<BTreeMap<String, String>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(BTreeMap::new());
    }
    let states: BTreeMap<String, String> = if text.starts_with('{') {
        serde_json::from_str(text).map_err(|e| e.to_string())?
    } else {
        let mut out = BTreeMap::new();
        for pair in text.split(',') {
            let (k, v) = pair
                .trim()
                .split_once('=')
                .ok_or("Use property=value, separated by commas")?;
            if out.insert(k.trim().into(), v.trim().into()).is_some() {
                return Err("Duplicate block property".into());
            }
        }
        out
    };
    if states
        .iter()
        .any(|(k, v)| !property_token(k) || !property_token(v))
    {
        return Err("Invalid block property name or value".into());
    }
    Ok(states)
}
#[derive(Clone, Default)]
pub struct Catalog {
    pub names: Arc<Vec<String>>,
    pub properties: Arc<Vec<BTreeMap<String, String>>>,
    pub matches: Vec<Option<usize>>,
}
static VISUAL_ENABLED: AtomicBool = AtomicBool::new(false);
pub fn visual_enabled() -> bool {
    VISUAL_ENABLED.load(Ordering::Acquire)
}
pub fn set_visual_enabled(on: bool) {
    VISUAL_ENABLED.store(on, Ordering::Release);
}
static ACTIVE: AtomicBool = AtomicBool::new(false);
static REVISION: AtomicU64 = AtomicU64::new(1);
static CONFIG_SERIAL: AtomicU64 = AtomicU64::new(1);
pub fn config_serial() -> u64 {
    CONFIG_SERIAL.load(Ordering::Acquire)
}
static CONFIG: RwLock<Option<Arc<Config>>> = RwLock::new(None);
static CATALOG: RwLock<Option<Arc<Catalog>>> = RwLock::new(None);
pub fn active() -> bool {
    ACTIVE.load(Ordering::Acquire)
}
pub fn set_active(on: bool) -> bool {
    ACTIVE.swap(on, Ordering::AcqRel) != on
}
pub fn revision() -> u64 {
    REVISION.load(Ordering::Acquire)
}
pub fn config() -> Arc<Config> {
    if let Some(c) = CONFIG.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        return c.clone();
    }
    let mut c = CONFIG.write().unwrap_or_else(|e| e.into_inner());
    c.get_or_insert_with(|| Arc::new(Config::default())).clone()
}
pub fn set_config(c: Config) -> Result<(), String> {
    let c = Arc::new(c.validated()?);
    let old_config = config();
    let changed_rules = old_config.rules != c.rules;
    let rescan = changed_rules
        || old_config.context_radius != c.context_radius
        || old_config.context_outlines != c.context_outlines
        || old_config.context_border_width != c.context_border_width
        || old_config.scan_radius != c.scan_radius
        || old_config.finder_enabled != c.finder_enabled;
    *CONFIG.write().unwrap_or_else(|e| e.into_inner()) = Some(c.clone());
    CONFIG_SERIAL.fetch_add(1, Ordering::AcqRel);
    if changed_rules {
        let mut cat = CATALOG.write().unwrap_or_else(|e| e.into_inner());
        if let Some(old) = cat.as_ref() {
            *cat = Some(Arc::new(Catalog {
                names: old.names.clone(),
                properties: old.properties.clone(),
                matches: compile_matches(&old.names, &old.properties, &c),
            }));
        }
    }
    if rescan {
        REVISION.fetch_add(1, Ordering::AcqRel);
    }
    Ok(())
}
pub fn catalog() -> Arc<Catalog> {
    if let Some(c) = CATALOG.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        return c.clone();
    }
    let mut c = CATALOG.write().unwrap_or_else(|e| e.into_inner());
    c.get_or_insert_with(|| Arc::new(Catalog::default()))
        .clone()
}
fn compile_catalog(
    names: Vec<String>,
    properties: Vec<BTreeMap<String, String>>,
    cfg: &Config,
) -> Catalog {
    let matches = compile_matches(&names, &properties, cfg);
    Catalog {
        names: Arc::new(names),
        properties: Arc::new(properties),
        matches,
    }
}
fn compile_matches(
    names: &[String],
    properties: &[BTreeMap<String, String>],
    cfg: &Config,
) -> Vec<Option<usize>> {
    names
        .iter()
        .enumerate()
        .map(|(id, name)| {
            cfg.rules.iter().position(|r| {
                r.enabled
                    && r.id == *name
                    && r.states
                        .iter()
                        .all(|(k, v)| properties.get(id).and_then(|p| p.get(k)) == Some(v))
            })
        })
        .collect()
}
pub fn set_catalog(names: Vec<String>, properties: Vec<BTreeMap<String, String>>) {
    let c = config();
    *CATALOG.write().unwrap_or_else(|e| e.into_inner()) =
        Some(Arc::new(compile_catalog(names, properties, &c)));
    REVISION.fetch_add(1, Ordering::AcqRel);
}
pub fn filter_snapshot(blocks: &mut [u32], light: &mut [u8], mask: &[bool]) {
    for b in blocks {
        if !mask.get(*b as usize).copied().unwrap_or(true) {
            *b = 0
        }
    }
    light.fill(255);
}
pub fn filter(blocks: &mut [u32], light: &mut [u8], base_y: i32, border: i32, side: usize) {
    if !active() || side == 0 {
        return;
    }
    let cfg = config();
    let cat = catalog();
    filter_received(blocks, light, &cat, &cfg, base_y, border, side);
}
fn filter_received(
    blocks: &mut [u32],
    light: &mut [u8],
    cat: &Catalog,
    cfg: &Config,
    base_y: i32,
    border: i32,
    side: usize,
) {
    if cfg.normal_terrain() || side == 0 {
        return;
    }
    filter_with(blocks, cat, cfg, base_y, border, side);
    light.fill(255);
}
fn filter_with(
    blocks: &mut [u32],
    cat: &Catalog,
    cfg: &Config,
    base_y: i32,
    border: i32,
    side: usize,
) {
    if cfg.normal_terrain() || side == 0 {
        return;
    }
    for (i, block) in blocks.iter_mut().enumerate() {
        let Some(selected) = cat.matches.get(*block as usize) else {
            continue;
        };
        let y = base_y + (i / (side * side)) as i32 - border;
        let keep = selected
            .and_then(|r| cfg.rules.get(r))
            .is_some_and(|r| y >= r.min_y && y <= r.max_y);
        if !keep {
            *block = 0
        }
    }
}
#[derive(Default, Debug)]
pub struct Activation {
    pub active: bool,
    pub down: bool,
}
impl Activation {
    pub fn event(&mut self, mode: ActivationMode, pressed: bool, playing: bool) -> bool {
        let before = self.active;
        let edge = pressed && !self.down;
        self.down = pressed && playing;
        match mode {
            ActivationMode::Hold => self.active = pressed && playing,
            ActivationMode::Toggle => {
                if edge && playing {
                    self.active = !self.active
                }
            }
        }
        before != self.active
    }
    pub fn cancel(&mut self) -> bool {
        let was = self.active;
        self.active = false;
        self.down = false;
        was
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracer_migration_keeps_duplicate_profile_fields_rejected() {
        assert!(
            Config::from_json(r#"{"awareness":{"tracer_items":true,"tracer_items":false}}"#)
                .is_err()
        );
    }
    #[test]
    fn entity_tracer_targets_are_independent_saved_preferences() {
        let text = r#"{"schema":1,"esp_players":false,"esp_mobs":true,"esp_items":false,"awareness":{"entity_tracers":true,"tracer_players":true,"tracer_mobs":false,"tracer_items":true}}"#;
        let cfg = Config::from_json(text).expect("independent tracer targets must be accepted");
        let saved: serde_json::Value = serde_json::from_str(&cfg.to_json().unwrap()).unwrap();
        assert_eq!(saved["awareness"]["tracer_players"], true);
        assert_eq!(saved["awareness"]["tracer_mobs"], false);
        assert_eq!(saved["awareness"]["tracer_items"], true);
        assert!(!cfg.esp_players && cfg.esp_mobs && !cfg.esp_items);
        assert_eq!(Config::from_json(&cfg.to_json().unwrap()).unwrap(), cfg);
    }
    #[test]
    fn legacy_native_tracers_keep_player_and_mob_coverage_without_overriding_new_flags() {
        let legacy = Config::from_json(r#"{"awareness":{"entity_tracers":true}}"#).unwrap();
        assert!(legacy.awareness.tracer_players && legacy.awareness.tracer_mobs);
        assert!(!legacy.awareness.tracer_items);
        let fresh = Config::from_json("{}").unwrap();
        assert!(fresh.awareness.tracer_players);
        assert!(!fresh.awareness.tracer_mobs && !fresh.awareness.tracer_items);
        let explicit = Config::from_json(
            r#"{"awareness":{"tracer_players":false,"tracer_mobs":false,"tracer_items":false}}"#,
        )
        .unwrap();
        assert!(
            !explicit.awareness.tracer_players
                && !explicit.awareness.tracer_mobs
                && !explicit.awareness.tracer_items
        );
    }
    #[test]
    fn textured_borders_leave_face_center_open_and_keep_profiles_compatible() {
        let c = Config::default();
        assert!((c.texture_visibility() - 25.).abs() < 0.0001);
        assert_eq!(c.context_opacity, 255);
        let legacy = Config::from_json(r#"{"schema":1,"context_opacity":100}"#).unwrap();
        assert!((legacy.texture_visibility() - 25.).abs() < 0.0001);
        assert_eq!(legacy.context_opacity, 100);
        let strips = c.terrain_frame_rects();
        let area: f32 = strips.iter().map(|r| (r[2] - r[0]) * (r[3] - r[1])).sum();
        assert!((area - 0.25).abs() < 0.00001);
        assert!(
            strips
                .iter()
                .all(|r| !(r[0] < 0.5 && r[2] > 0.5 && r[1] < 0.5 && r[3] > 0.5))
        );
        assert!(
            strips
                .iter()
                .all(|r| r.iter().all(|v| (0.0..=1.0).contains(v)))
        );
        for width in [f32::NAN, f32::INFINITY, 0.0, 50.1] {
            assert!(
                Config {
                    context_border_width: width,
                    ..c.clone()
                }
                .validated()
                .is_err()
            );
        }
        let tuned = Config {
            context_border_width: 10.0,
            ..c
        };
        assert_eq!(
            Config::from_json(&tuned.to_json().unwrap())
                .unwrap()
                .context_border_width,
            10.0
        );
    }
    #[test]
    fn texture_slider_endpoints_and_legacy_values_are_lossless() {
        let mut c = Config::default();
        for percent in [0., 1., 25., 50., 75., 99., 100.] {
            c.set_texture_visibility(percent).unwrap();
            assert!((c.texture_visibility() - percent).abs() < 0.001);
            assert_eq!(c.normal_terrain(), percent == 100.);
            let area: f32 = c
                .terrain_frame_rects()
                .iter()
                .map(|r| (r[2] - r[0]) * (r[3] - r[1]))
                .sum();
            assert!((area - (percent / 100.).max(0.003996)).abs() < 0.00001);
            c.clone().validated().unwrap();
        }
        c.context_outlines = false;
        assert!(!c.normal_terrain());
        for invalid in [-0.1, 100.1, f32::NAN, f32::INFINITY] {
            let before = c.clone();
            assert!(c.set_texture_visibility(invalid).is_err());
            assert_eq!(c, before);
        }
        let legacy = Config::from_json(r#"{"schema":1,"context_border_width":6.25}"#).unwrap();
        assert_eq!(legacy.context_border_width, 6.25);
        assert!((legacy.texture_visibility() - 23.4375).abs() < 0.001);
    }
    #[test]
    fn normal_texture_endpoint_preserves_received_blocks_and_lighting() {
        let mut c = Config::default();
        c.set_texture_visibility(100.).unwrap();
        let cat = compile_catalog(
            vec![
                "minecraft:air".into(),
                "minecraft:stone".into(),
                "minecraft:diamond_ore".into(),
            ],
            vec![],
            &c,
        );
        let source = [1, 2, 1, 0, 1, 2, 1, 1];
        let illumination = [0, 19, 35, 94, 128, 159, 200, 255];
        let mut blocks = source;
        let mut light = illumination;
        filter_received(&mut blocks, &mut light, &cat, &c, 0, 0, 2);
        assert_eq!(blocks, source);
        assert_eq!(light, illumination);
        c.set_texture_visibility(25.).unwrap();
        filter_received(&mut blocks, &mut light, &cat, &c, 0, 0, 2);
        assert_eq!(blocks, [0, 2, 0, 0, 0, 2, 0, 0]);
        assert_eq!(light, [255; 8]);
    }
    #[test]
    fn default_is_toggle_with_terrain_context_and_all_usual_ores() {
        let c = Config::default();
        assert_eq!(c.activation, ActivationMode::Toggle);
        assert!(c.context_outlines);
        assert!(c.rules.iter().any(|r| r.id == "minecraft:ancient_debris"));
        assert!(
            c.rules
                .iter()
                .any(|r| r.id == "minecraft:deepslate_copper_ore")
        );
    }
    #[test]
    fn hold_releases_even_outside_gameplay() {
        let mut a = Activation::default();
        assert!(a.event(ActivationMode::Hold, true, true));
        assert!(a.active);
        assert!(a.event(ActivationMode::Hold, false, false));
        assert!(!a.active);
    }
    #[test]
    fn toggle_ignores_repeat_and_release() {
        let mut a = Activation::default();
        a.event(ActivationMode::Toggle, true, true);
        assert!(a.active);
        assert!(!a.event(ActivationMode::Toggle, true, true));
        a.event(ActivationMode::Toggle, false, true);
        assert!(a.active);
        a.event(ActivationMode::Toggle, true, true);
        assert!(!a.active);
    }
    #[test]
    fn import_rejects_invalid_bounds_and_schema() {
        let c = Config {
            schema: 9,
            ..Default::default()
        };
        assert!(c.validated().is_err());
        let mut c = Config::default();
        c.rules.push(BlockRule {
            min_y: 80,
            max_y: 20,
            ..BlockRule::new("stone")
        });
        assert!(c.validated().is_err());
    }
    #[test]
    fn rule_bounds_include_edges_and_negative_coordinates() {
        let r = BlockRule {
            radius: 3,
            min_y: -64,
            max_y: 10,
            ..BlockRule::new("stone")
        };
        assert!(r.contains([-1, -64, -1], [-0.5, 0., -0.5]));
        assert!(!r.contains([4, 0, 0], [0., 0., 0.]));
        assert!(!r.contains([0, 11, 0], [0., 0., 0.]));
    }
    #[test]
    fn duplicate_block_ids_are_rejected_instead_of_shadowing_rules() {
        let mut c = Config::default();
        c.rules.push(c.rules[0].clone());
        assert!(c.validated().is_err());
    }
    #[test]
    fn profile_round_trip_and_state_csv() {
        let c = Config::default();
        assert_eq!(Config::from_json(&c.to_json().unwrap()).unwrap(), c);
        assert_eq!(parse_states(" lit=true, facing=north ").unwrap().len(), 2);
        assert!(parse_states("lit=true,lit=false").is_err());
        assert_eq!(
            normalize_id("DIAMOND_ORE").unwrap(),
            "minecraft:diamond_ore"
        );
    }
    #[test]
    fn catalog_and_filter_respect_properties_and_y_without_touching_unknown_ids() {
        let mut c = Config::default();
        let mut r = BlockRule::new("minecraft:redstone_ore");
        r.states = parse_states("lit=true").unwrap();
        r.min_y = 0;
        r.max_y = 0;
        c.rules = vec![r];
        let names = vec![
            "minecraft:air".into(),
            "minecraft:redstone_ore".into(),
            "minecraft:redstone_ore".into(),
        ];
        let cat = compile_catalog(
            names,
            vec![
                BTreeMap::new(),
                parse_states("lit=false").unwrap(),
                parse_states("lit=true").unwrap(),
            ],
            &c,
        );
        assert_eq!(cat.matches, vec![None, None, Some(0)]);
        let mut blocks = vec![1, 2, 99, 2, 2, 2, 2, 2];
        filter_with(&mut blocks, &cat, &c, 0, 0, 2);
        assert_eq!(blocks, vec![0, 2, 99, 2, 0, 0, 0, 0]);
    }
}
