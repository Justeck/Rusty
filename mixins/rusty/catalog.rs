// Rusty module metadata and local preferences. Copyright 2026 Justeck, Apache-2.0.
use serde::{Deserialize, Deserializer, Serialize, de};
use std::collections::BTreeSet;
use std::fmt;

/// Stable names from the live activation predicate; the list never lists itself.
pub fn enabled_names(mut active: impl FnMut(&str) -> bool) -> Vec<&'static str> {
    let mut names: Vec<_> = MODULES
        .iter()
        .filter(|m| m.id != "enabled_mods" && active(m.id))
        .map(|m| m.name)
        .collect();
    names.sort_unstable();
    names
}

pub const MAX_QUERY_CHARS: usize = 128;
pub const MAX_FAVORITES: usize = 128;
pub const MAX_RECENT: usize = 16;
// Persist physical scales so existing layouts keep their exact size. UI percentages
// are relative to the compact baseline: former 70% is now 100%.
pub const DEFAULT_MENU_SCALE: f32 = 0.7;
pub const MIN_MENU_SCALE: f32 = 0.35;
pub const MAX_MENU_SCALE: f32 = 2.1;
pub fn menu_percent(scale: f32) -> f32 {
    scale / DEFAULT_MENU_SCALE * 100.0
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Classic,
    Graphite,
    Copper,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ClientPreferences {
    pub theme: Theme,
    pub menu_scale: f32,
    pub hud_layout: super::rusty_hud_layout::HudLayout,
    pub reduced_motion: bool,
    pub favorites: Vec<String>,
    pub recent: Vec<String>,
}

impl Default for ClientPreferences {
    fn default() -> Self {
        Self {
            theme: Theme::Classic,
            menu_scale: DEFAULT_MENU_SCALE,
            hud_layout: Default::default(),
            reduced_motion: false,
            favorites: Vec::new(),
            recent: Vec::new(),
        }
    }
}
impl ClientPreferences {
    pub fn validate(&self) -> Result<(), String> {
        if !self.menu_scale.is_finite()
            || !(MIN_MENU_SCALE..=MAX_MENU_SCALE).contains(&self.menu_scale)
        {
            return Err("Menu size must be 50-300%".into());
        }
        self.hud_layout.validate()?;
        validate_ids(&self.favorites, MAX_FAVORITES, "favorites")?;
        validate_ids(&self.recent, MAX_RECENT, "recent")
    }
    pub fn validated(self) -> Result<Self, String> {
        self.validate()?;
        Ok(self)
    }
    /// Records a known module, most recent first. Unknown IDs do not mutate state.
    pub fn visit(&mut self, id: &str) -> bool {
        if module(id).is_none() {
            return false;
        }
        clean_ids(&mut self.recent, MAX_RECENT);
        self.recent.retain(|item| item != id);
        self.recent.insert(0, id.to_owned());
        self.recent.truncate(MAX_RECENT);
        true
    }
    /// Returns the resulting favorite state. Unknown IDs are never added.
    pub fn toggle_favorite(&mut self, id: &str) -> bool {
        if module(id).is_none() {
            return false;
        }
        clean_ids(&mut self.favorites, MAX_FAVORITES);
        if self.is_favorite(id) {
            self.favorites.retain(|item| item != id);
            false
        } else if self.favorites.len() < MAX_FAVORITES {
            self.favorites.push(id.to_owned());
            true
        } else {
            false
        }
    }
    pub fn is_favorite(&self, id: &str) -> bool {
        self.favorites.iter().any(|x| x == id)
    }
}

fn validate_ids(ids: &[String], limit: usize, field: &str) -> Result<(), String> {
    if ids.len() > limit {
        return Err(format!("At most {limit} {field} are supported"));
    }
    let mut seen = BTreeSet::new();
    for id in ids {
        if module(id).is_none() {
            return Err(format!("Unknown module ID in {field}: {id}"));
        }
        if !seen.insert(id) {
            return Err(format!("Duplicate module ID in {field}: {id}"));
        }
    }
    Ok(())
}

fn clean_ids(ids: &mut Vec<String>, limit: usize) {
    let mut seen = BTreeSet::new();
    ids.retain(|id| module(id).is_some() && seen.insert(id.clone()));
    ids.truncate(limit);
}

// Bound sequence allocation during deserialization, before semantic validation.
fn bounded_ids<'de, D: Deserializer<'de>, const LIMIT: usize>(
    deserializer: D,
) -> Result<Vec<String>, D::Error> {
    struct Ids<const N: usize>;
    impl<'de, const N: usize> de::Visitor<'de> for Ids<N> {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "at most {N} module IDs")
        }
        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut ids = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(N));
            while let Some(id) = seq.next_element::<String>()? {
                if ids.len() == N {
                    return Err(de::Error::custom(format!(
                        "Too many module IDs (maximum {N})"
                    )));
                }
                if id.len() > 64 {
                    return Err(de::Error::custom("Module ID is too long"));
                }
                ids.push(id);
            }
            Ok(ids)
        }
    }
    deserializer.deserialize_seq(Ids::<LIMIT>)
}

impl<'de> Deserialize<'de> for ClientPreferences {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(default)]
        struct Data {
            theme: Theme,
            menu_scale: f32,
            hud_layout: super::rusty_hud_layout::HudLayout,
            reduced_motion: bool,
            #[serde(deserialize_with = "bounded_ids::<_, MAX_FAVORITES>")]
            favorites: Vec<String>,
            #[serde(deserialize_with = "bounded_ids::<_, MAX_RECENT>")]
            recent: Vec<String>,
        }
        impl Default for Data {
            fn default() -> Self {
                Self {
                    theme: Theme::Classic,
                    menu_scale: DEFAULT_MENU_SCALE,
                    hud_layout: Default::default(),
                    reduced_motion: false,
                    favorites: Vec::new(),
                    recent: Vec::new(),
                }
            }
        }
        let data = Data::deserialize(deserializer)?;
        Self {
            theme: data.theme,
            menu_scale: data.menu_scale,
            hud_layout: data.hud_layout,
            reduced_motion: data.reduced_motion,
            favorites: data.favorites,
            recent: data.recent,
        }
        .validated()
        .map_err(de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Render,
    World,
    Player,
    Movement,
    Interface,
    Navigation,
    Camera,
    Inventory,
    Combat,
    Utility,
    Building,
}

impl Category {
    pub const ALL: &'static [Self] = &[
        Self::Render,
        Self::World,
        Self::Player,
        Self::Movement,
        Self::Interface,
        Self::Navigation,
        Self::Camera,
        Self::Inventory,
        Self::Combat,
        Self::Utility,
        Self::Building,
    ];
    pub const fn id(self) -> &'static str {
        match self {
            Self::Render => "render",
            Self::World => "world",
            Self::Player => "player",
            Self::Movement => "movement",
            Self::Interface => "interface",
            Self::Navigation => "navigation",
            Self::Camera => "camera",
            Self::Inventory => "inventory",
            Self::Combat => "combat",
            Self::Utility => "utility",
            Self::Building => "building",
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Render => "Render",
            Self::World => "World",
            Self::Player => "Player",
            Self::Movement => "Movement",
            Self::Interface => "Interface",
            Self::Navigation => "Navigation",
            Self::Camera => "Camera",
            Self::Inventory => "Inventory",
            Self::Combat => "Combat",
            Self::Utility => "Utility",
            Self::Building => "Building",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChoiceOption {
    pub id: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingKind {
    Bool {
        default: bool,
    },
    Integer {
        min: i64,
        max: i64,
        step: i64,
        default: i64,
    },
    Decimal {
        min: f64,
        max: f64,
        step: f64,
        default: f64,
    },
    Choice {
        options: &'static [ChoiceOption],
        default: &'static str,
    },
    Action,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettingSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub aliases: &'static [&'static str],
    pub kind: SettingKind,
}

#[derive(Debug, PartialEq)]
pub struct ModuleSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: Category,
    pub aliases: &'static [&'static str],
    pub settings: &'static [SettingSpec],
    pub keybind_id: Option<&'static str>,
    pub default_key: Option<&'static str>,
    pub default_enabled: bool,
}

const ACTIVATION_OPTIONS: &[ChoiceOption] = &[
    ChoiceOption {
        id: "toggle",
        label: "Toggle",
    },
    ChoiceOption {
        id: "hold",
        label: "Hold",
    },
];

const fn boolean(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    default: bool,
) -> SettingSpec {
    SettingSpec {
        id,
        label,
        description,
        aliases: &[],
        kind: SettingKind::Bool { default },
    }
}
const fn integer(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    min: i64,
    max: i64,
    default: i64,
) -> SettingSpec {
    SettingSpec {
        id,
        label,
        description,
        aliases: &[],
        kind: SettingKind::Integer {
            min,
            max,
            step: 1,
            default,
        },
    }
}
const fn decimal(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    range: [f64; 2],
    step: f64,
    default: f64,
) -> SettingSpec {
    SettingSpec {
        id,
        label,
        description,
        aliases: &[],
        kind: SettingKind::Decimal {
            min: range[0],
            max: range[1],
            step,
            default,
        },
    }
}
const EDIT_RULES: SettingSpec = SettingSpec {
    id: "edit_rules",
    label: "Target Rules",
    description: "Open the advanced editor for block IDs, labels, groups, states, colors, height bounds, per-rule radius, enabled state, outlines and tracers.",
    aliases: &["block filters", "ore list", "min y", "max y"],
    kind: SettingKind::Action,
};
const XRAY_SETTINGS: &[SettingSpec] = &[
    SettingSpec {
        id: "activation",
        label: "Activation",
        description: "Toggle X-ray with a press or show it only while its key is held.",
        aliases: &["hold mode", "toggle mode"],
        kind: SettingKind::Choice {
            options: ACTIVATION_OPTIONS,
            default: "toggle",
        },
    },
    boolean(
        "context_outlines",
        "Textured Context",
        "Retain ordinary block textures around selected targets.",
        true,
    ),
    integer(
        "context_radius",
        "Context Radius",
        "Maximum context distance in blocks.",
        1,
        48,
        18,
    ),
    integer(
        "context_opacity",
        "Context Opacity",
        "Opacity of ordinary textured context, from transparent to solid.",
        0,
        255,
        255,
    ),
    decimal(
        "texture_visibility",
        "Texture Visibility",
        "Visible area of each ordinary block face, in percent.",
        [0.0, 100.0],
        1.0,
        25.0,
    ),
    EDIT_RULES,
];
const FINDER_SETTINGS: &[SettingSpec] = &[
    boolean(
        "finder_enabled",
        "Finder Enabled",
        "Enable target boxes, tracer lines and target readouts independently of X-ray terrain rendering.",
        false,
    ),
    boolean(
        "auto_scan",
        "Automatic Scan",
        "Refresh received target locations automatically.",
        true,
    ),
    integer(
        "scan_radius",
        "Scan Radius",
        "Maximum horizontal search radius in loaded blocks.",
        1,
        128,
        64,
    ),
    integer(
        "scan_interval",
        "Scan Interval",
        "Minimum ticks between automatic scans.",
        1,
        60,
        5,
    ),
    integer(
        "max_targets",
        "Maximum Targets",
        "Bound the number of target results retained.",
        1,
        4096,
        1024,
    ),
    boolean(
        "hud_counts",
        "Target Counts",
        "Include scanned target counts in the status HUD.",
        false,
    ),
    boolean(
        "block_outlines",
        "Block Outlines",
        "Draw outlines around selected received target blocks.",
        true,
    ),
    boolean(
        "tracers",
        "Tracers",
        "Draw lines toward scanned targets that allow tracers.",
        true,
    ),
    integer(
        "tracer_limit",
        "Tracer Limit",
        "Maximum number of target tracer lines.",
        0,
        12,
        12,
    ),
    decimal(
        "line_width",
        "Line Width",
        "Width of target overlay lines.",
        [0.5, 4.0],
        0.1,
        1.2,
    ),
    EDIT_RULES,
];
const ENTITY_SETTINGS: &[SettingSpec] = &[
    boolean(
        "esp_players",
        "Players",
        "Highlight received player entities.",
        true,
    ),
    boolean(
        "esp_mobs",
        "Mobs",
        "Highlight received living non-player entities.",
        false,
    ),
    boolean(
        "esp_items",
        "Items",
        "Highlight received dropped item entities.",
        false,
    ),
];
const ENTITY_TRACER_SETTINGS: &[SettingSpec] = &[
    boolean(
        "tracer_players",
        "Players",
        "Draw lines to received players; independent of Entity ESP.",
        true,
    ),
    boolean(
        "tracer_mobs",
        "Mobs",
        "Draw lines to received living non-player entities; independent of Entity ESP.",
        false,
    ),
    boolean(
        "tracer_items",
        "Dropped Items",
        "Draw lines to received dropped items; independent of Entity ESP.",
        false,
    ),
];
const ZOOM_SETTINGS: &[SettingSpec] = &[decimal(
    "zoom_factor",
    "Zoom Factor",
    "Camera magnification while the zoom key is held.",
    [1.0, 16.0],
    0.25,
    4.0,
)];
const FREECAM_SETTINGS: &[SettingSpec] = &[
    decimal(
        "freecam_speed",
        "Camera Speed",
        "Movement speed of the detached local camera.",
        [0.05, 8.0],
        0.05,
        1.0,
    ),
    boolean(
        "freecam_noclip",
        "Camera No Clip",
        "Allow the detached camera to pass through received block geometry.",
        true,
    ),
];
const HUD_SETTINGS: &[SettingSpec] = &[boolean(
    "hud_counts",
    "Target Counts",
    "Include scanned target counts in the status HUD.",
    false,
)];

const TRAIL_SETTINGS: &[SettingSpec] = &[
    decimal(
        "sample_distance",
        "Point Spacing",
        "Record another observed body position after this distance, in blocks.",
        [0.5, 16.0],
        0.5,
        2.0,
    ),
    decimal(
        "teleport_distance",
        "Break Distance",
        "Start a new trail across large jumps; server teleports always break the trail.",
        [16.0, 128.0],
        1.0,
        64.0,
    ),
];
const WAYPOINT_SETTINGS: &[SettingSpec] = &[SettingSpec {
    id: "add_waypoint",
    label: "Save Current Position",
    description: "Save this body position in the current world or server and dimension. Manage with /rusty waypoint.",
    aliases: &["home", "marker"],
    kind: SettingKind::Action,
}];
pub static MODULES: &[ModuleSpec] = &[
    ModuleSpec {
        id: "xray",
        name: "X-Ray",
        description: "Reveal selected received blocks while retaining textured world context.",
        category: Category::Render,
        aliases: &["xray", "ore vision", "ore finder"],
        settings: XRAY_SETTINGS,
        keybind_id: Some("xray"),
        default_key: Some("key.keyboard.x"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "finder",
        name: "Finder",
        description: "Scan loaded blocks for enabled target rules and render their locations.",
        category: Category::World,
        aliases: &["block search", "scanner", "target finder"],
        settings: FINDER_SETTINGS,
        keybind_id: Some("finder"),
        default_key: Some("key.keyboard.v"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "entity_esp",
        name: "Entity ESP",
        description: "Highlight received entities with selectable player, mob and item filters.",
        category: Category::Render,
        aliases: &["entity highlights", "entity radar"],
        settings: ENTITY_SETTINGS,
        keybind_id: Some("entity_esp"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "fullbright",
        name: "Fullbright",
        description: "Brighten the local world view for visibility in dark areas.",
        category: Category::Render,
        aliases: &["night vision", "brightness", "gamma"],
        settings: &[],
        keybind_id: Some("fullbright"),
        default_key: Some("key.keyboard.b"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "no_fog",
        name: "No Fog",
        description: "Remove local distance fog without revealing unreceived terrain.",
        category: Category::Render,
        aliases: &["fog removal", "clear view"],
        settings: &[],
        keybind_id: Some("no_fog"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "clear_weather",
        name: "Clear Weather",
        description: "Hide local weather effects while server weather remains authoritative.",
        category: Category::Render,
        aliases: &["no rain", "weather visuals"],
        settings: &[],
        keybind_id: Some("clear_weather"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "zoom",
        name: "Zoom",
        description: "Magnify the camera view while holding the zoom key.",
        category: Category::Camera,
        aliases: &["magnify", "telescope"],
        settings: ZOOM_SETTINGS,
        keybind_id: Some("zoom"),
        default_key: Some("key.keyboard.c"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "coordinates",
        name: "Coordinates",
        description: "Show current coordinates and dimension in the local HUD.",
        category: Category::Navigation,
        aliases: &["position", "coords", "location"],
        settings: &[],
        keybind_id: Some("coordinates"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "auto_sprint",
        name: "Auto Sprint",
        description: "Request normal sprinting while walking forward.",
        category: Category::Movement,
        aliases: &["autosprint", "run"],
        settings: &[],
        keybind_id: Some("auto_sprint"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "no_hurt_camera",
        name: "No Hurt Camera",
        description: "Suppress local camera shake from damage.",
        category: Category::Player,
        aliases: &["hurtcam", "damage shake"],
        settings: &[],
        keybind_id: Some("no_hurt_camera"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "freecam",
        name: "Freecam",
        description: "Detach the local camera while the player body continues normal simulation.",
        category: Category::Camera,
        aliases: &["free camera", "detached camera"],
        settings: FREECAM_SETTINGS,
        keybind_id: Some("freecam"),
        default_key: Some("key.keyboard.u"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "status_hud",
        name: "Status HUD",
        description: "Show active Rusty state and optional target counts.",
        category: Category::Interface,
        aliases: &["overlay", "status strip", "module status"],
        settings: HUD_SETTINGS,
        keybind_id: Some("status_hud"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: true,
    },
    ModuleSpec {
        id: "enabled_mods",
        name: "Enabled Mods",
        description: "List active Rusty features vertically in the top-left. Move or resize it with its HUD layout controls.",
        category: Category::Interface,
        aliases: &["array list", "active modules", "enabled list"],
        settings: &[],
        keybind_id: Some("enabled_mods"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "compass",
        name: "Compass",
        description: "Show your cardinal direction and yaw using the current player rotation.",
        category: Category::Navigation,
        aliases: &["heading", "direction", "north"],
        settings: &[],
        keybind_id: Some("compass"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "waypoints",
        name: "Waypoints",
        description: "Save local markers, scoped to each world/server and dimension, with distance and bearing HUD.",
        category: Category::Navigation,
        aliases: &["home", "markers", "coordinates", "beacon"],
        settings: WAYPOINT_SETTINGS,
        keybind_id: Some("waypoints"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "breadcrumbs",
        name: "Breadcrumbs",
        description: "Trace observed body movement with a bounded trail; world changes and teleports break the path.",
        category: Category::Navigation,
        aliases: &["trail", "path", "route"],
        settings: TRAIL_SETTINGS,
        keybind_id: Some("breadcrumbs"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "equipment_hud",
        name: "Equipment HUD",
        description: "Show received armor and held-item durability with a configurable low-durability warning.",
        category: Category::Inventory,
        aliases: &["armor", "tools", "damage", "durability"],
        settings: &[integer(
            "warning_percent",
            "Low Durability",
            "Warn when remaining equipment durability is at or below this percentage.",
            1,
            50,
            20,
        )],
        keybind_id: Some("equipment_hud"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "environment_hud",
        name: "Environment HUD",
        description: "Show the current received biome and local block and sky light; missing data stays unavailable.",
        category: Category::World,
        aliases: &["biome", "local light", "sky light", "block light"],
        settings: &[],
        keybind_id: Some("environment_hud"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "container_preview",
        name: "Container Preview",
        description: "List embedded contents supplied with the held item, without opening it or querying world chests.",
        category: Category::Inventory,
        aliases: &["shulker", "contents", "item preview", "box"],
        settings: &[integer(
            "preview_rows",
            "Preview Rows",
            "Maximum visible item rows from received top-level container contents.",
            1,
            9,
            5,
        )],
        keybind_id: Some("container_preview"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "chunk_borders",
        name: "Chunk Borders",
        description: "Show the current chunk boundaries and section heights.",
        category: Category::World,
        aliases: &["chunk grid"],
        settings: &[],
        keybind_id: Some("chunk_borders"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "light_overlay",
        name: "Light Risk",
        description: "Mark dark and skylit full-block floors using received light. Not a complete spawn predictor.",
        category: Category::World,
        aliases: &["light level spawn risk"],
        settings: &[integer(
            "overlay_radius",
            "Overlay Radius",
            "Nearby received observations, in blocks.",
            4,
            24,
            12,
        )],
        keybind_id: Some("light_overlay"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "storage_esp",
        name: "Storage ESP",
        description: "Outline received nearby storage block entities without reading hidden contents.",
        category: Category::Render,
        aliases: &["chest barrel shulker"],
        settings: &[integer(
            "overlay_radius",
            "Overlay Radius",
            "Nearby received observations, in blocks.",
            4,
            24,
            12,
        )],
        keybind_id: Some("storage_esp"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "trajectories",
        name: "Trajectories",
        description: "Estimate supported bow and thrown-item paths with native block, entity and fluid physics. Random spread and future movement remain unknown.",
        category: Category::Render,
        aliases: &["bow pearl prediction"],
        settings: &[integer(
            "trajectory_steps",
            "Prediction Steps",
            "Bounded simulation steps; unknown terrain stops prediction.",
            20,
            200,
            100,
        )],
        keybind_id: Some("trajectories"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "target_info",
        name: "Target Info",
        description: "Inspect the received block at the crosshair, properties and distance within normal reach.",
        category: Category::World,
        aliases: &["crosshair block properties"],
        settings: &[],
        keybind_id: Some("target_info"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "portal_coordinates",
        name: "Portal Coordinates",
        description: "Display the corresponding Overworld or Nether XZ coordinates; no portal or destination scan.",
        category: Category::Navigation,
        aliases: &["nether calculator"],
        settings: &[],
        keybind_id: Some("portal_coordinates"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "speed_hud",
        name: "Speed HUD",
        description: "Show observed horizontal body speed, resetting on discontinuities.",
        category: Category::Interface,
        aliases: &["movement meters per second"],
        settings: &[],
        keybind_id: Some("speed_hud"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "clock_hud",
        name: "World Clock",
        description: "Show received world time as a 24-hour clock.",
        category: Category::Interface,
        aliases: &["day time"],
        settings: &[],
        keybind_id: Some("clock_hud"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "entity_tracers",
        name: "Entity Tracers",
        description: "Choose independent player, mob and dropped-item tracers within the search radius.",
        category: Category::Render,
        aliases: &["entity pointers lines"],
        settings: ENTITY_TRACER_SETTINGS,
        keybind_id: Some("entity_tracers"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "safe_walk",
        name: "Safe Walk",
        description: "Request normal crouching near a ledge while grounded; respects jumping and collision shapes.",
        category: Category::Movement,
        aliases: &["edge sneak"],
        settings: &[],
        keybind_id: Some("safe_walk"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "auto_jump",
        name: "Auto Jump",
        description: "Request a normal jump over a received obstacle with clear headroom.",
        category: Category::Movement,
        aliases: &["parkour"],
        settings: &[],
        keybind_id: Some("auto_jump"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "auto_tool",
        name: "Auto Tool",
        description: "Choose a suitable hotbar tool while you mine; preserve drops and protect low durability.",
        category: Category::Inventory,
        aliases: &["pickaxe shovel axe"],
        settings: &[integer(
            "tool_min_remaining",
            "Protect Tools",
            "Skip tools below this remaining durability percentage.",
            0,
            100,
            5,
        )],
        keybind_id: Some("auto_tool"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "trigger_bot",
        name: "Trigger Bot",
        description: "Attack a living crosshair target through the ordinary client action at full cooldown and normal reach.",
        category: Category::Combat,
        aliases: &["crosshair attack"],
        settings: &[boolean(
            "trigger_players",
            "Include Players",
            "Also allow ordinary crosshair attacks against received players. Team and friend filtering is not provided.",
            false,
        )],
        keybind_id: Some("trigger_bot"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "auto_eat",
        name: "Auto Eat",
        description: "Eat ordinary food from the hotbar below a hunger threshold; cancel on user actions and never open a block.",
        category: Category::Player,
        aliases: &["food hunger"],
        settings: &[integer(
            "eat_below",
            "Hunger Threshold",
            "Start eating at or below these food points (20 is full).",
            1,
            19,
            16,
        )],
        keybind_id: Some("auto_eat"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "armor_assist",
        name: "Armor Assist",
        description: "Fill empty armor slots from known inventory equipment with ordinary clicks. Preserve worn gear and skip binding curses.",
        category: Category::Inventory,
        aliases: &["auto armor equipment"],
        settings: &[],
        keybind_id: Some("armor_assist"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "refill",
        name: "Auto Refill",
        description: "Refill an observed depleted hotbar slot with an exact matching main-inventory stack. Never clear a slot or blindly retry a correction.",
        category: Category::Inventory,
        aliases: &["restock replenish"],
        settings: &[],
        keybind_id: Some("refill"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "build_planner",
        name: "Build Planner",
        description: "Preview a bounded cuboid plan and explicitly place one reachable held block at a time through normal client actions.",
        category: Category::Building,
        aliases: &["blueprint hollow box construction"],
        settings: &[
            integer(
                "build_width",
                "Width",
                "Plan width in blocks. Total plan is bounded to 4096 cells.",
                1,
                16,
                5,
            ),
            integer("build_depth", "Depth", "Plan depth in blocks.", 1, 16, 5),
            integer("build_height", "Height", "Plan height in blocks.", 1, 16, 3),
            boolean(
                "build_hollow",
                "Hollow",
                "Keep only side walls plus the chosen floor and roof.",
                true,
            ),
            boolean(
                "build_floor",
                "Floor",
                "Include the bottom face in a hollow plan.",
                true,
            ),
            boolean(
                "build_roof",
                "Roof",
                "Include the top face in a hollow plan.",
                false,
            ),
            SettingSpec {
                id: "build_anchor",
                label: "Set Origin",
                description: "Set the plan origin from the current received view.",
                aliases: &["anchor"],
                kind: SettingKind::Action,
            },
            SettingSpec {
                id: "build_place",
                label: "Place Next",
                description: "Explicitly request one reachable placement with the held block. Never starts continuous building.",
                aliases: &["place one"],
                kind: SettingKind::Action,
            },
        ],
        keybind_id: Some("build_planner"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "auto_fish",
        name: "Auto Fish",
        description: "Reel an owned fishing hook after a received bite. Cast once yourself to begin; respects normal use cooldown.",
        category: Category::Player,
        aliases: &["fishing rod bite"],
        settings: &[
            integer(
                "fish_delay",
                "Bite Delay",
                "Wait this many client ticks after the received bite, and the ordinary use cooldown.",
                2,
                20,
                4,
            ),
            boolean(
                "fish_recast",
                "Recast",
                "After confirmed hook removal, cast again. User actions or a menu cancel the sequence.",
                false,
            ),
        ],
        keybind_id: Some("auto_fish"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "auto_totem",
        name: "Auto Totem",
        description: "Move an ordinary hotbar totem into an empty offhand with a normal hand swap; preserve shields and other offhand items.",
        category: Category::Inventory,
        aliases: &["offhand survival"],
        settings: &[],
        keybind_id: Some("auto_totem"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "death_markers",
        name: "Death Markers",
        description: "Save one waypoint after an observed death in the current world and dimension; keep your selected marker unchanged.",
        category: Category::Navigation,
        aliases: &["death location recovery"],
        settings: &[],
        keybind_id: Some("death_markers"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "area_clear",
        name: "Area Clear",
        description: "Select two block corners and explicitly clear their box, highest layer first. Normal guarded mining only; protected and unreachable blocks remain with an explanation.",
        category: Category::World,
        aliases: &["excavate excavation demolish destroy cuboid quarry region selection"],
        settings: &[
            SettingSpec {
                id: "clear_first",
                label: "Set First Corner",
                description: "Aim at the first block before opening the menu. The selection belongs only to this world and dimension.",
                aliases: &[],
                kind: SettingKind::Action,
            },
            SettingSpec {
                id: "clear_second",
                label: "Set Second Corner",
                description: "Aim at the opposite block corner. Both end blocks are included. Maximum 32 blocks per axis / 32768 cells.",
                aliases: &[],
                kind: SettingKind::Action,
            },
            SettingSpec {
                id: "clear_info",
                label: "Selection Details",
                description: "Review the selected box before enabling. Clearing never mines outside it; the top needs a safely walkable approach.",
                aliases: &[],
                kind: SettingKind::Action,
            },
            SettingSpec {
                id: "clear_selection",
                label: "Clear Selection",
                description: "Forget both corners without breaking anything. Selections also clear on world or dimension changes.",
                aliases: &[],
                kind: SettingKind::Action,
            },
            integer(
                "clear_health",
                "Minimum Health",
                "Stop after observed damage or below this health, in half-heart units.",
                1,
                20,
                12,
            ),
            integer(
                "clear_food",
                "Minimum Food",
                "Stop below this food level.",
                0,
                20,
                10,
            ),
            integer(
                "clear_durability",
                "Tool Reserve",
                "Never use a tool at or below this remaining durability percentage.",
                1,
                100,
                10,
            ),
            boolean(
                "clear_preview",
                "Selection and Route Preview",
                "Show the twelve edges of the selected box and checked movement/mining route.",
                true,
            ),
        ],
        keybind_id: Some("area_clear"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "auto_mine",
        name: "Auto Mine",
        description: "Explicitly mine a chosen resource using received terrain and normal controls. Conservative routes stop on danger, damage or manual input; no automatic resume.",
        category: Category::World,
        aliases: &["miner resource ore bot pathfind tunnel"],
        settings: &[
            SettingSpec {
                id: "mine_resource",
                label: "Resource",
                description: "Choose an ore preset, or Custom for a block selected from your crosshair.",
                aliases: &["target ore"],
                kind: SettingKind::Choice {
                    options: &[
                        ChoiceOption {
                            id: "0",
                            label: "Coal",
                        },
                        ChoiceOption {
                            id: "1",
                            label: "Iron",
                        },
                        ChoiceOption {
                            id: "2",
                            label: "Copper",
                        },
                        ChoiceOption {
                            id: "3",
                            label: "Gold",
                        },
                        ChoiceOption {
                            id: "4",
                            label: "Redstone",
                        },
                        ChoiceOption {
                            id: "5",
                            label: "Lapis",
                        },
                        ChoiceOption {
                            id: "6",
                            label: "Diamond",
                        },
                        ChoiceOption {
                            id: "7",
                            label: "Emerald",
                        },
                        ChoiceOption {
                            id: "8",
                            label: "Quartz",
                        },
                        ChoiceOption {
                            id: "9",
                            label: "Nether gold",
                        },
                        ChoiceOption {
                            id: "10",
                            label: "Ancient debris",
                        },
                        ChoiceOption {
                            id: "11",
                            label: "Custom",
                        },
                    ],
                    default: "6",
                },
            },
            SettingSpec {
                id: "mine_aimed",
                label: "Use Aimed Block",
                description: "Close the menu, aim at a safe natural block, then use this button to choose its registered block type as the custom resource.",
                aliases: &[],
                kind: SettingKind::Action,
            },
            integer(
                "mine_radius",
                "Search Radius",
                "Horizontal blocks searched. Only terrain already received by the client is used.",
                8,
                32,
                24,
            ),
            integer(
                "mine_goal",
                "Block Goal",
                "Stop after this many observed resource blocks. Drops depend on the normal tool and server rules.",
                1,
                256,
                16,
            ),
            integer(
                "mine_health",
                "Minimum Health",
                "Stop below this health in half-heart units, or immediately after any observed damage.",
                1,
                20,
                12,
            ),
            integer(
                "mine_food",
                "Minimum Food",
                "Stop below this food level; Auto Mine does not eat for you.",
                0,
                20,
                10,
            ),
            integer(
                "mine_durability",
                "Tool Reserve",
                "Stop using tools at or below this remaining durability percentage.",
                1,
                100,
                10,
            ),
            boolean(
                "mine_tunnels",
                "Access Tunnels",
                "Allow short access tunnels through safe ordinary terrain. Never break containers, unknown blocks or the player’s floor.",
                true,
            ),
            boolean(
                "mine_preview",
                "Route Preview",
                "Show the checked route in the world. Route estimates include walking and mining time.",
                true,
            ),
        ],
        keybind_id: Some("auto_mine"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "waypoint_walk",
        name: "Waypoint Walk",
        description: "Explicitly walk to the selected waypoint through loaded terrain. Manual input or opening a menu stops walking.",
        category: Category::Navigation,
        aliases: &["goto pathfind navigate route"],
        settings: &[],
        keybind_id: Some("waypoint_walk"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "minimap",
        name: "Minimap",
        description: "Use Horizon’s native map of received terrain, with bounded tile work and automatic surface/underground layers.",
        category: Category::Navigation,
        aliases: &["radar terrain map north"],
        settings: &[
            integer(
                "minimap_size",
                "Map Size",
                "Map side in GUI pixels.",
                64,
                160,
                96,
            ),
            decimal(
                "minimap_scale",
                "Map Scale",
                "Blocks per image pixel; larger values show a wider area.",
                [0.5, 4.0],
                0.5,
                1.0,
            ),
            boolean(
                "minimap_rotate",
                "Rotate Map",
                "Rotate the map with your heading instead of north up.",
                false,
            ),
            boolean(
                "minimap_round",
                "Round Map",
                "Use a round map rather than a square.",
                true,
            ),
        ],
        keybind_id: Some("minimap"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "fps_hud",
        name: "Frame Rate",
        description: "Show Horizon’s sampled FPS, minimum and one percent low frame rate.",
        category: Category::Interface,
        aliases: &["fps performance frames"],
        settings: &[],
        keybind_id: Some("fps_hud"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
    ModuleSpec {
        id: "clicks_hud",
        name: "Click Counter",
        description: "Show ordinary attack and use clicks from the native input counter over the last second.",
        category: Category::Interface,
        aliases: &["cps clicks per second"],
        settings: &[],
        keybind_id: Some("clicks_hud"),
        default_key: Some("key.keyboard.unknown"),
        default_enabled: false,
    },
];
pub fn module(id: &str) -> Option<&'static ModuleSpec> {
    MODULES.iter().find(|m| m.id == id)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Collection {
    #[default]
    All,
    Favorites,
    Recent,
    Enabled,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SearchHit {
    pub module: &'static ModuleSpec,
    pub setting: Option<&'static SettingSpec>,
    pub score: u32,
}

/// Returns one best hit per module. Every token must occur in its module fields
/// or one matching setting. Empty recent searches retain newest-first order;
/// other ties use stable module IDs, never registration or hash-map order.
pub fn search(
    query: &str,
    category: Option<Category>,
    collection: Collection,
    prefs: &ClientPreferences,
) -> Vec<SearchHit> {
    search_active(query, category, collection, prefs, |_| false)
}

pub fn search_active(
    query: &str,
    category: Option<Category>,
    collection: Collection,
    prefs: &ClientPreferences,
    active: impl Fn(&str) -> bool,
) -> Vec<SearchHit> {
    let query: String = query
        .chars()
        .take(MAX_QUERY_CHARS)
        .collect::<String>()
        .to_lowercase();
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let phrase = tokens.join(" ");
    let mut hits = Vec::new();
    for m in MODULES {
        if category.is_some_and(|c| c != m.category) {
            continue;
        }
        match collection {
            Collection::Enabled if !active(m.id) => continue,
            Collection::Favorites if !prefs.is_favorite(m.id) => continue,
            Collection::Recent if !prefs.recent.iter().any(|id| id == m.id) => continue,
            _ => {}
        }
        if tokens.is_empty() {
            hits.push(SearchHit {
                module: m,
                setting: None,
                score: 0,
            });
            continue;
        }
        let mut best = candidate(m, None, &tokens, &phrase);
        for setting in m.settings {
            if let Some(hit) = candidate(m, Some(setting), &tokens, &phrase)
                && best.is_none_or(|old| hit.score > old.score)
            {
                best = Some(hit);
            }
        }
        if let Some(hit) = best {
            hits.push(hit);
        }
    }
    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| {
                if collection == Collection::Recent {
                    let index = |id| {
                        prefs
                            .recent
                            .iter()
                            .position(|item| item == id)
                            .unwrap_or(usize::MAX)
                    };
                    index(a.module.id).cmp(&index(b.module.id))
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then_with(|| a.module.id.cmp(b.module.id))
    });
    hits
}

fn field_score(field: &str, token: &str, base: u32) -> u32 {
    let field = field.to_lowercase();
    if field == token {
        base + 2
    } else if field.starts_with(token) {
        base + 1
    } else if field.contains(token) {
        base
    } else {
        0
    }
}

fn candidate(
    m: &'static ModuleSpec,
    setting: Option<&'static SettingSpec>,
    tokens: &[&str],
    phrase: &str,
) -> Option<SearchHit> {
    let mut sum = 0;
    let mut minimum = u32::MAX;
    let mut setting_matched = false;
    for token in tokens {
        let title_score = field_score(m.name, token, 80).max(field_score(m.id, token, 80));
        let alias_score = m
            .aliases
            .iter()
            .map(|s| field_score(s, token, 60))
            .max()
            .unwrap_or(0);
        let description_score = field_score(m.description, token, 10);
        let control_score = setting
            .map(|s| {
                field_score(s.label, token, 40)
                    .max(field_score(s.id, token, 40))
                    .max(
                        s.aliases
                            .iter()
                            .map(|alias| field_score(alias, token, 40))
                            .max()
                            .unwrap_or(0),
                    )
                    .max(field_score(s.description, token, 10))
            })
            .unwrap_or(0);
        let module_score = title_score.max(alias_score).max(description_score);
        let score = module_score.max(control_score);
        if score == 0 {
            return None;
        }
        setting_matched |= control_score > module_score;
        sum += score;
        minimum = minimum.min(score);
    }
    // A setting-only candidate must add a match; otherwise keep the module hit.
    if setting.is_some() && !setting_matched {
        return None;
    }
    let title = m.name.to_lowercase();
    let tier = if title == phrase || m.id == phrase {
        9
    } else if title.starts_with(phrase) || m.id.starts_with(phrase) {
        8
    } else if minimum >= 80 {
        7
    } else if minimum >= 60 {
        6
    } else if minimum >= 40 {
        4
    } else {
        1
    };
    Some(SearchHit {
        module: m,
        setting: setting.filter(|_| setting_matched),
        score: tier * 100_000 + sum,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn enabled_mods_is_a_separate_opt_in_interface_feature() {
        let feature = module("enabled_mods").expect("Enabled Mods must be in the module menu");
        assert_eq!(feature.name, "Enabled Mods");
        assert_eq!(feature.category, Category::Interface);
        assert!(!feature.default_enabled);
    }
    #[test]
    fn compact_menu_default_and_existing_physical_sizes_are_preserved() {
        assert_eq!(ClientPreferences::default().menu_scale, 0.7);
        for (scale, percent) in [(0.35, 50.0), (0.7, 100.0), (2.1, 300.0)] {
            assert!((menu_percent(scale) - percent).abs() < 0.001);
        }
        let missing: ClientPreferences = serde_json::from_str("{}").unwrap();
        assert_eq!(missing.menu_scale, 0.7);
        for scale in [0.35, 0.5, 0.7, 1.0, 2.0, 2.1] {
            let saved = serde_json::json!({"menu_scale": scale});
            let p: ClientPreferences = serde_json::from_value(saved).unwrap();
            assert_eq!(p.menu_scale, scale);
            assert_eq!(
                serde_json::from_str::<ClientPreferences>(&serde_json::to_string(&p).unwrap())
                    .unwrap()
                    .menu_scale,
                scale
            );
        }
    }
    use super::*;

    fn ids(hits: Vec<SearchHit>) -> Vec<&'static str> {
        hits.into_iter().map(|h| h.module.id).collect()
    }

    // Missing aliases or dropping description matching breaks user discovery.
    #[test]
    fn aliases_and_descriptions_find_real_modules() {
        let p = ClientPreferences::default();
        assert_eq!(
            ids(search("ore vision", None, Collection::All, &p)),
            ["xray"]
        );
        assert_eq!(
            ids(search("received entities", None, Collection::All, &p)),
            ["entity_esp", "entity_tracers", "storage_esp"]
        );
    }

    // Losing setting labels or treating tokens as OR loses the focused pane.
    #[test]
    fn setting_hit_identifies_the_control_and_all_tokens_are_required() {
        let p = ClientPreferences::default();
        let hits = search("x-ray context opacity", None, Collection::All, &p);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].module.id, "xray");
        assert_eq!(hits[0].setting.unwrap().id, "context_opacity");
        assert!(search("x-ray nonexistent", None, Collection::All, &p).is_empty());
    }

    // Changing ranking to registration order would put finder before xray.
    #[test]
    fn title_precedes_alias_and_setting_matches_with_deterministic_ties() {
        let p = ClientPreferences::default();
        let hits = search("finder", None, Collection::All, &p);
        assert_eq!(hits[0].module.id, "finder");
        let prefix = search("x-r", None, Collection::All, &p);
        assert_eq!(prefix[0].module.id, "xray");
        assert_eq!(
            ids(search("", None, Collection::All, &p)),
            ids(search(" ", None, Collection::All, &p))
        );
    }

    // Ignoring any filter leaks unrelated modules into favorites/categories.
    #[test]
    fn collection_and_category_filters_intersect() {
        let mut p = ClientPreferences::default();
        p.toggle_favorite("xray");
        p.toggle_favorite("auto_sprint");
        assert_eq!(
            ids(search(
                "",
                Some(Category::Render),
                Collection::Favorites,
                &p
            )),
            ["xray"]
        );
        p.visit("xray");
        p.visit("zoom");
        p.visit("xray");
        assert_eq!(
            ids(search("", None, Collection::Recent, &p)),
            ["xray", "zoom"]
        );
        assert!(search("sprint", Some(Category::Render), Collection::All, &p).is_empty());
    }

    // Repeated visits must move a module to front, not grow the persisted list.
    #[test]
    fn preference_operations_ignore_unknown_ids_and_do_not_duplicate() {
        let mut p = ClientPreferences::default();
        assert!(!p.visit("planned-flight"));
        assert!(!p.toggle_favorite("planned-flight"));
        assert!(p.toggle_favorite("xray"));
        assert!(p.is_favorite("xray"));
        assert!(!p.toggle_favorite("xray"));
        for _ in 0..100 {
            p.visit("xray");
            p.visit("zoom");
        }
        assert_eq!(p.recent, ["zoom", "xray"]);
        assert!(p.validate().is_ok());
    }

    // Unvalidated serialized profiles must not inject unknown/duplicate IDs.
    #[test]
    fn validation_rejects_unknown_duplicate_and_oversized_lists() {
        for p in [
            ClientPreferences {
                favorites: vec!["unknown".into()],
                ..Default::default()
            },
            ClientPreferences {
                favorites: vec!["xray".into(), "xray".into()],
                ..Default::default()
            },
            ClientPreferences {
                recent: vec!["zoom".into(), "zoom".into()],
                ..Default::default()
            },
            ClientPreferences {
                favorites: vec!["xray".into(); 129],
                ..Default::default()
            },
            ClientPreferences {
                recent: vec!["xray".into(); 17],
                ..Default::default()
            },
        ] {
            assert!(
                p.validate().is_err(),
                "accepted malformed preferences: {p:?}"
            );
        }
        let huge = serde_json::json!({ "recent": vec!["xray"; 17] });
        assert!(serde_json::from_value::<ClientPreferences>(huge).is_err());
    }

    // Missing fields in legacy files must keep old config readable.
    #[test]
    fn legacy_preferences_default_and_round_trip() {
        let default: ClientPreferences = serde_json::from_str("{}").unwrap();
        assert_eq!(default, ClientPreferences::default());
        let p = ClientPreferences {
            theme: Theme::Copper,
            reduced_motion: true,
            favorites: vec!["xray".into()],
            recent: vec!["zoom".into()],
            ..Default::default()
        };
        let encoded = serde_json::to_string(&p).unwrap();
        assert_eq!(
            serde_json::from_str::<ClientPreferences>(&encoded)
                .unwrap()
                .validated()
                .unwrap(),
            p
        );
        assert!(serde_json::from_str::<ClientPreferences>(r#"{"theme":"unknown"}"#).is_err());
    }

    // Byte slicing a multi-byte query panics; unbounded queries evade input caps.
    #[test]
    fn unicode_query_is_capped_without_splitting_utf8() {
        let p = ClientPreferences::default();
        let prefix = "xray".to_owned() + &" ".repeat(124);
        assert_eq!(
            ids(search(&(prefix + "absent"), None, Collection::All, &p)),
            ["xray"]
        );
        assert!(search(&"🧭".repeat(200), None, Collection::All, &p).is_empty());
        assert_eq!(
            ids(search("  FuLlBrIgHt  ", None, Collection::All, &p)),
            ["fullbright"]
        );
    }

    // Metadata mistakes must not generate out-of-range sliders or empty choices.
    #[test]
    fn available_controls_have_valid_ranges_and_unambiguous_ids() {
        assert!(module("finder").is_some());
        assert!(module("flight").is_none());
        for m in MODULES {
            let mut keys = std::collections::BTreeSet::new();
            for s in m.settings {
                assert!(keys.insert(s.id), "duplicate setting {} in {}", s.id, m.id);
                match s.kind {
                    SettingKind::Integer {
                        min,
                        max,
                        step,
                        default,
                    } => assert!(min <= default && default <= max && step > 0),
                    SettingKind::Decimal {
                        min,
                        max,
                        step,
                        default,
                    } => assert!(
                        min.is_finite()
                            && max.is_finite()
                            && default.is_finite()
                            && min <= default
                            && default <= max
                            && step.is_finite()
                            && step > 0.0
                    ),
                    SettingKind::Choice { options, default } => {
                        assert!(options.iter().any(|o| o.id == default))
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod enabled_tests {
    use super::*;
    #[test]
    fn enabled_collection_tracks_live_state_including_transient_modules() {
        let prefs = ClientPreferences::default();
        let active = |id: &str| {
            matches!(
                id,
                "xray" | "zoom" | "freecam" | "waypoint_walk" | "fullbright"
            )
        };
        let hits = search_active("", None, Collection::Enabled, &prefs, active);
        assert_eq!(hits.len(), 5);
        assert!(hits.iter().all(|h| active(h.module.id)));
        assert!(search_active("", None, Collection::Enabled, &prefs, |_| false).is_empty());
        assert_eq!(
            search_active("zoom", None, Collection::Enabled, &prefs, active).len(),
            1
        );
        assert!(
            search_active(
                "zoom",
                Some(Category::Render),
                Collection::Enabled,
                &prefs,
                |_| false
            )
            .is_empty()
        );
    }
}
