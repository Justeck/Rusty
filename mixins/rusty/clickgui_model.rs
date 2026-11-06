// Rusty native module browser. Copyright 2026 Justeck, Apache-2.0.
use super::rusty_catalog::{self as catalog, Category, Collection, SettingKind};
use std::collections::VecDeque;

const MAX_INTENTS: usize = 64;
// Minecraft's eight-pixel font gets separate name/category baselines and space below.
pub const MODULE_ROW_HEIGHT: f32 = 40.0;
pub const MODULE_ROW_PITCH: f32 = 43.0;
pub const MODULE_TITLE_Y: f32 = 7.0;
pub const MODULE_CATEGORY_Y: f32 = 25.0;

/// AltGr commonly arrives as Control+Alt and remains ordinary text input.
pub fn text_shortcut(control: bool, super_key: bool, alt: bool) -> bool {
    (control || super_key) && !alt
}

#[derive(Clone, Debug)]
pub struct SaveState<T> {
    pub value: T,
    pub dirty: bool,
    pub blocked: bool,
}
impl<T> SaveState<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            dirty: false,
            blocked: false,
        }
    }
    pub fn stage(&mut self, value: T) {
        self.value = value;
        self.dirty = true;
        self.blocked = false;
    }
    pub fn failed(&mut self) {
        self.blocked = true;
    }
    pub fn retry(&mut self) {
        self.blocked = false;
    }
    pub fn committed(&mut self) {
        self.dirty = false;
        self.blocked = false;
    }
    pub fn should_save(&self, elapsed_ms: f32, force: bool) -> bool {
        self.dirty && !self.blocked && (force || elapsed_ms >= 200.0)
    }
}

pub fn mouse_binding(button: i32) -> Option<String> {
    Some(match button {
        0 => "key.mouse.left".into(),
        1 => "key.mouse.right".into(),
        2 => "key.mouse.middle".into(),
        3..=31 => format!("key.mouse.{}", button + 1),
        _ => return None,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    ToggleModule(&'static str),
    ToggleBool(&'static str),
    Numeric(&'static str, f64),
    Nudge(&'static str, f64),
    Activation,
    MiningResource,
    MiningAimed,
    Favorite(&'static str),
    Visit(&'static str),
    Theme,
    Motion,
    ResetHud(&'static str),
    HudLayout(super::rusty_hud_layout::HudLayout),
    DefaultSetting(&'static str),
    RestoreDefaults,
    Bind(&'static str, String),
    Advanced,
    AddWaypoint,
    BuildAnchor,
    BuildPlace,
    Paste,
    Copy(String),
    Flush,
}

impl Intent {
    pub fn persists_after_external_close(&self) -> bool {
        match self {
            Self::Numeric(..)
            | Self::Nudge(..)
            | Self::ToggleBool(..)
            | Self::Activation
            | Self::MiningResource
            | Self::MiningAimed
            | Self::Favorite(..)
            | Self::Visit(..)
            | Self::Theme
            | Self::Motion
            | Self::ResetHud(..)
            | Self::HudLayout(..)
            | Self::DefaultSetting(..)
            | Self::RestoreDefaults
            | Self::Bind(..)
            | Self::Flush => true,
            Self::ToggleModule(id) => !matches!(
                *id,
                "xray" | "freecam" | "zoom" | "waypoint_walk" | "auto_mine"
            ),
            _ => false,
        }
    }
}

#[cfg(test)]
mod action_cancel_tests {
    use super::Intent;
    #[test]
    fn queued_waypoint_motion_is_discarded_when_the_screen_closes_externally() {
        assert!(!Intent::ToggleModule("waypoint_walk").persists_after_external_close());
        assert!(!Intent::BuildPlace.persists_after_external_close());
        assert!(!Intent::BuildAnchor.persists_after_external_close());
        assert!(Intent::ToggleModule("compass").persists_after_external_close());
    }
}

#[derive(Clone, Debug, Default)]
pub struct TextInput {
    pub value: String,
    pub caret: usize,
    pub anchor: usize,
}
impl TextInput {
    pub fn insert(&mut self, s: &str) {
        let mut chars: Vec<char> = self.value.chars().collect();
        let a = self.caret.min(self.anchor).min(chars.len());
        let b = self.caret.max(self.anchor).min(chars.len());
        let available = catalog::MAX_QUERY_CHARS.saturating_sub(chars.len() - (b - a));
        let added: Vec<char> = s
            .chars()
            .filter(|c| !c.is_control() && *c != '\u{7f}' && *c != '§')
            .take(available)
            .collect();
        let count = added.len();
        chars.splice(a..b, added);
        self.value = chars.into_iter().collect();
        self.caret = a + count;
        self.anchor = self.caret;
    }
    pub fn selected(&self) -> String {
        self.value
            .chars()
            .skip(self.caret.min(self.anchor))
            .take(self.caret.abs_diff(self.anchor))
            .collect()
    }
    pub fn backspace(&mut self) {
        if self.caret == self.anchor {
            self.anchor = self.caret.saturating_sub(1);
        }
        self.insert("");
    }
    pub fn delete(&mut self) {
        if self.caret == self.anchor {
            self.anchor = (self.caret + 1).min(self.value.chars().count());
        }
        self.insert("");
    }
    pub fn move_to(&mut self, at: usize, select: bool) {
        self.caret = at.min(self.value.chars().count());
        if !select {
            self.anchor = self.caret;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Search,
    Select(&'static str),
    Toggle(&'static str),
    Favorite(&'static str),
    Collection(Collection),
    CategoryPicker,
    Category(Option<Category>),
    Theme,
    Motion,
    ResetMenuScale,
    DefaultSetting(&'static str),
    DefaultBind(&'static str),
    RestoreDefaults,
    ConfirmDefaults,
    CancelDefaults,
    Bool(&'static str),
    Choice(&'static str),
    Numeric(&'static str, [f32; 4], SettingKind),
    Step(&'static str, f64),
    Bind(&'static str),
    ClearBind(&'static str),
    ResetHud(&'static str),
    EditHud,
    Advanced,
    AddWaypoint,
    BuildAnchor,
    BuildPlace,
    Help(String),
    Close,
}
#[derive(Clone, Debug)]
pub struct Hit {
    pub rect: [f32; 4],
    pub action: Action,
}

pub struct Model {
    pub confirm_defaults: bool,
    pub render_scale: f32,
    pub record_controls: bool,
    pub open: bool,
    pub close_requested: bool,
    pub query: TextInput,
    pub search_focus: bool,
    pub selected: &'static str,
    pub category: Option<Category>,
    pub collection: Collection,
    pub list_scroll: f32,
    pub detail_scroll: f32,
    pub list_rect: [f32; 4],
    pub detail_rect: [f32; 4],
    pub list_height: f32,
    pub detail_height: f32,
    pub hits: Vec<Hit>,
    pub keyboard_hit: Option<usize>,
    pub dragging: Option<(&'static str, [f32; 4], SettingKind)>,
    pub capture: Option<&'static str>,
    pub categories_open: bool,
    pub reveal_setting: bool,
    pub help: Option<String>,
    pub intents: VecDeque<Intent>,
    pub status: String,
    pub error: bool,
}
impl Default for Model {
    fn default() -> Self {
        Self {
            confirm_defaults: false,
            render_scale: 1.0,
            record_controls: true,
            open: false,
            close_requested: false,
            query: TextInput::default(),
            search_focus: true,
            selected: "xray",
            category: None,
            collection: Collection::All,
            list_scroll: 0.0,
            detail_scroll: 0.0,
            list_rect: [0.0; 4],
            detail_rect: [0.0; 4],
            list_height: 0.0,
            detail_height: 0.0,
            hits: Vec::new(),
            keyboard_hit: None,
            dragging: None,
            capture: None,
            categories_open: false,
            reveal_setting: false,
            help: None,
            intents: VecDeque::new(),
            status: String::new(),
            error: false,
        }
    }
}
impl Model {
    pub fn character(&mut self, ch: char, control: bool, super_key: bool, alt: bool) {
        if self.open
            && self.search_focus
            && self.capture.is_none()
            && self.help.is_none()
            && !self.confirm_defaults
            && !text_shortcut(control, super_key, alt)
        {
            self.query.insert(&ch.to_string());
            self.query_changed();
        }
    }
    pub fn enqueue(&mut self, intent: Intent) -> bool {
        if !self.open || self.close_requested {
            return false;
        }
        if let Intent::Numeric(id, _) = &intent
            && let Some(Intent::Numeric(last, _)) = self.intents.back()
            && id == last
        {
            self.intents.pop_back();
            self.intents.push_back(intent);
            return true;
        }
        if self.intents.len() == MAX_INTENTS {
            self.status = "Input queue is full; try again after the next frame".into();
            self.error = true;
            return false;
        }
        self.intents.push_back(intent);
        true
    }
    pub fn deactivate(&mut self) {
        self.confirm_defaults = false;
        self.open = false;
        self.close_requested = false;
        self.intents.clear();
        self.dragging = None;
        self.capture = None;
        self.hits.clear();
        self.help = None;
        self.categories_open = false;
        self.keyboard_hit = None;
    }
    pub fn clear_binding(&mut self, name: &'static str) {
        self.capture = None;
        self.enqueue(Intent::Bind(name, clear_binding().into()));
        self.enqueue(Intent::Flush);
    }
    pub fn query_changed(&mut self) {
        self.list_scroll = 0.0;
        self.hits.clear();
        self.keyboard_hit = None;
        self.reveal_setting = true;
    }
}

pub fn intersect(rect: [f32; 4], clip: [f32; 4]) -> Option<[f32; 4]> {
    if !rect.iter().chain(clip.iter()).all(|n| n.is_finite()) {
        return None;
    }
    let x = rect[0].max(clip[0]);
    let y = rect[1].max(clip[1]);
    let right = (rect[0] + rect[2]).min(clip[0] + clip[2]);
    let bottom = (rect[1] + rect[3]).min(clip[1] + clip[3]);
    (right > x && bottom > y).then_some([x, y, right - x, bottom - y])
}
pub fn inside(at: [f32; 2], rect: [f32; 4]) -> bool {
    at[0] >= rect[0] && at[1] >= rect[1] && at[0] < rect[0] + rect[2] && at[1] < rect[1] + rect[3]
}
pub fn numeric_value(kind: SettingKind, fraction: f64) -> Option<f64> {
    if !fraction.is_finite() {
        return None;
    }
    let (min, max, step) = match kind {
        SettingKind::Integer { min, max, step, .. } => (min as f64, max as f64, step as f64),
        SettingKind::Decimal { min, max, step, .. } => (min, max, step),
        _ => return None,
    };
    if fraction >= 1.0 {
        return Some(max);
    }
    let raw = min + fraction.clamp(0.0, 1.0) * (max - min);
    Some((min + ((raw - min) / step).round() * step).clamp(min, max))
}

pub fn nudge_value(kind: SettingKind, current: f64, steps: f64) -> Option<f64> {
    if !current.is_finite() || !steps.is_finite() {
        return None;
    }
    let (min, max, step) = match kind {
        SettingKind::Integer { min, max, step, .. } => (min as f64, max as f64, step as f64),
        SettingKind::Decimal { min, max, step, .. } => (min, max, step),
        _ => return None,
    };
    numeric_value(kind, (current + steps * step - min) / (max - min))
}

pub fn opening_opacity(elapsed_ms: f32, reduced_motion: bool) -> f32 {
    if reduced_motion {
        1.0
    } else if elapsed_ms.is_finite() {
        (elapsed_ms / 120.0).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reset_confirmation_blocks_background_typing_and_is_cleared_on_close() {
        let mut ui = Model {
            open: true,
            search_focus: true,
            confirm_defaults: true,
            ..Default::default()
        };
        ui.character('x', false, false, false);
        assert!(ui.query.value.is_empty());
        ui.deactivate();
        assert!(!ui.confirm_defaults);
    }
    #[test]
    fn external_close_keeps_pending_settings_and_discards_gameplay_intents() {
        let mut ui = Model {
            open: true,
            ..Default::default()
        };
        ui.enqueue(Intent::Numeric("scan_radius", 17.0));
        ui.enqueue(Intent::ToggleModule("freecam"));
        ui.enqueue(Intent::ToggleModule("xray"));
        ui.enqueue(Intent::Favorite("finder"));
        let persist: Vec<_> = ui
            .intents
            .drain(..)
            .filter(Intent::persists_after_external_close)
            .collect();
        ui.deactivate();
        assert_eq!(
            persist,
            [
                Intent::Numeric("scan_radius", 17.0),
                Intent::Favorite("finder")
            ]
        );
        let mut pending = SaveState::new(64.0);
        pending.stage(17.0);
        assert!(pending.should_save(0.0, true));
        pending.failed();
        assert_eq!(pending.value, 17.0);
        assert!(pending.dirty);
        assert!(!pending.should_save(5000.0, false));
        pending.retry();
        assert!(pending.should_save(0.0, true));
    }
    #[test]
    fn multiple_nudges_remain_distinct_and_accumulate_against_current_value() {
        let kind = SettingKind::Decimal {
            min: 0.5,
            max: 4.0,
            step: 0.1,
            default: 1.2,
        };
        let mut ui = Model {
            open: true,
            ..Default::default()
        };
        ui.enqueue(Intent::Nudge("line_width", 1.0));
        ui.enqueue(Intent::Nudge("line_width", 1.0));
        assert_eq!(ui.intents.len(), 2);
        let mut value = 1.2;
        for intent in ui.intents {
            if let Intent::Nudge(_, steps) = intent {
                value = nudge_value(kind, value, steps).unwrap();
            }
        }
        assert!((value - 1.4).abs() < 1e-8);
        assert_eq!(nudge_value(kind, 4.0, 1.0), Some(4.0));
        assert_eq!(nudge_value(kind, 1.0, f64::NAN), None);
    }
    #[test]
    fn failed_save_retains_the_draft_and_requires_an_explicit_retry() {
        let mut state = SaveState::new(7);
        state.stage(9);
        assert!(state.should_save(250.0, false));
        state.failed();
        assert_eq!(state.value, 9);
        assert!(state.dirty);
        assert!(state.blocked);
        assert!(!state.should_save(10_000.0, false));
        state.retry();
        assert!(state.should_save(0.0, true));
        state.committed();
        assert!(!state.dirty);
        assert!(!state.should_save(10_000.0, true));
    }
    #[test]
    fn mouse_bindings_use_canonical_engine_names() {
        assert_eq!(mouse_binding(0).as_deref(), Some("key.mouse.left"));
        assert_eq!(mouse_binding(1).as_deref(), Some("key.mouse.right"));
        assert_eq!(mouse_binding(2).as_deref(), Some("key.mouse.middle"));
        assert_eq!(mouse_binding(3).as_deref(), Some("key.mouse.4"));
        assert_eq!(mouse_binding(4).as_deref(), Some("key.mouse.5"));
        assert_eq!(mouse_binding(-1), None);
        assert_eq!(mouse_binding(32), None);
    }
    #[test]
    fn opening_fade_is_bounded_and_reduced_motion_is_instant() {
        assert_eq!(opening_opacity(0.0, false), 0.0);
        assert_eq!(opening_opacity(60.0, false), 0.5);
        assert_eq!(opening_opacity(120.0, false), 1.0);
        assert_eq!(opening_opacity(999.0, false), 1.0);
        assert_eq!(opening_opacity(0.0, true), 1.0);
    }
    #[test]
    fn unicode_search_editing_replaces_selection_and_keeps_character_boundaries() {
        let mut t = TextInput::default();
        t.insert("café 🧭");
        t.move_to(4, false);
        t.backspace();
        assert_eq!(t.value, "caf 🧭");
        t.move_to(0, false);
        t.move_to(3, true);
        assert_eq!(t.selected(), "caf");
        t.insert("ore");
        assert_eq!(t.value, "ore 🧭");
        t.move_to(4, false);
        t.delete();
        assert_eq!(t.value, "ore ");
        t.insert(&"😀".repeat(200));
        assert_eq!(t.value.chars().count(), 128);
    }

    #[test]
    fn shortcut_character_echo_never_replaces_search_selection() {
        for (control, super_key) in [(true, false), (false, true)] {
            for ch in ['a', 'c', 'f', 'v'] {
                let mut ui = Model {
                    open: true,
                    ..Default::default()
                };
                ui.query.insert("fullbright");
                // Ctrl+A/F have already focused/select-all before charTyped;
                // Ctrl+C/V must also preserve this selection for copy/paste.
                ui.query.move_to(0, false);
                ui.query.move_to(10, true);
                ui.character(ch, control, super_key, false);
                assert_eq!(ui.query.value, "fullbright");
                assert_eq!(ui.query.selected(), "fullbright");
            }
        }
    }

    #[test]
    fn clipboard_paste_and_cut_do_not_add_a_literal_shortcut_character() {
        let mut ui = Model {
            open: true,
            ..Default::default()
        };
        ui.enqueue(Intent::Paste);
        ui.character('v', true, false, false);
        // The queued paste runs after the key's separate character callback.
        ui.query.insert("fullbright");
        assert_eq!(ui.query.value, "fullbright");
        ui.query.move_to(0, false);
        ui.query.move_to(10, true);
        let copied = ui.query.selected();
        ui.query.insert("");
        ui.character('x', true, false, false);
        assert_eq!(copied, "fullbright");
        assert!(ui.query.value.is_empty());
    }

    #[test]
    fn ordinary_unicode_and_altgr_characters_keep_text_input() {
        let mut ui = Model {
            open: true,
            ..Default::default()
        };
        for ch in "é你🧭".chars() {
            ui.character(ch, false, false, false);
        }
        for ch in "€@".chars() {
            ui.character(ch, true, false, true);
        }
        assert_eq!(ui.query.value, "é你🧭€@");
    }

    #[test]
    fn help_capture_and_inactive_search_do_not_receive_character_text() {
        let mut ui = Model {
            open: true,
            help: Some("Help".into()),
            ..Default::default()
        };
        ui.character('a', false, false, false);
        assert!(ui.query.value.is_empty());
        ui.help = None;
        ui.capture = Some("xray");
        ui.character('你', false, false, false);
        assert!(ui.query.value.is_empty());
        ui.capture = None;
        ui.search_focus = false;
        ui.character('v', true, false, false);
        assert!(ui.query.value.is_empty());
        ui.search_focus = true;
        ui.open = false;
        ui.character('é', false, false, false);
        assert!(ui.query.value.is_empty());
    }
    #[test]
    fn queue_is_bounded_and_coalesces_drag_values() {
        let mut m = Model {
            open: true,
            ..Default::default()
        };
        for n in 0..1000 {
            assert!(m.enqueue(Intent::Numeric("scan_radius", n as f64)));
        }
        assert_eq!(m.intents.len(), 1);
        assert_eq!(
            m.intents.back(),
            Some(&Intent::Numeric("scan_radius", 999.0))
        );
        for _ in 1..64 {
            assert!(m.enqueue(Intent::ToggleBool("hud_counts")));
        }
        assert!(!m.enqueue(Intent::ToggleBool("hud_counts")));
        assert_eq!(m.intents.len(), 64);
    }
    #[test]
    fn closing_or_losing_screen_discards_stale_input_state() {
        let mut m = Model {
            open: true,
            capture: Some("zoom"),
            ..Default::default()
        };
        m.enqueue(Intent::ToggleModule("xray"));
        m.dragging = Some((
            "zoom_factor",
            [1.0; 4],
            SettingKind::Decimal {
                min: 1.0,
                max: 16.0,
                step: 0.25,
                default: 4.0,
            },
        ));
        m.deactivate();
        assert!(m.intents.is_empty());
        assert!(m.capture.is_none());
        assert!(m.dragging.is_none());
        assert!(!m.enqueue(Intent::ToggleModule("xray")));
    }
    #[test]
    fn clipped_rows_cannot_receive_invisible_clicks() {
        let r = intersect([0.0, 5.0, 20.0, 20.0], [0.0, 10.0, 20.0, 10.0]).unwrap();
        assert!(!inside([5.0, 8.0], r));
        assert!(inside([5.0, 12.0], r));
        assert!(intersect([0.0, 40.0, 20.0, 20.0], [0.0, 10.0, 20.0, 10.0]).is_none());
        assert!(intersect([f32::NAN, 0.0, 20.0, 20.0], [0.0; 4]).is_none());
    }
    #[test]
    fn slider_reaches_endpoints_and_rejects_non_finite_input() {
        let k = SettingKind::Decimal {
            min: 0.05,
            max: 8.0,
            step: 0.05,
            default: 1.0,
        };
        assert_eq!(numeric_value(k, 0.0), Some(0.05));
        assert_eq!(numeric_value(k, 1.0), Some(8.0));
        assert_eq!(numeric_value(k, f64::NAN), None);
        assert_eq!(
            numeric_value(
                SettingKind::Integer {
                    min: 1,
                    max: 128,
                    step: 1,
                    default: 64
                },
                2.0
            ),
            Some(128.0)
        );
    }
}

#[cfg(test)]
mod minecraft_geometry_tests {
    use super::*;
    #[test]
    fn module_cards_reserve_padding_below_the_category() {
        const {
            assert!(MODULE_CATEGORY_Y - (MODULE_TITLE_Y + 8.0) >= 8.0);
            assert!(MODULE_ROW_HEIGHT - (MODULE_CATEGORY_Y + 8.0) >= 7.0);
            assert!(MODULE_ROW_PITCH - MODULE_ROW_HEIGHT >= 3.0);
        }
    }
    #[test]
    fn scaled_targets_and_slider_geometry_share_one_transform() {
        let transform = MenuTransform::new([800.0, 600.0], 1.5);
        let rect = [10.0, 20.0, 100.0, 25.0];
        let screen = transform.rect(rect);
        assert!(inside(transform.point([60.0, 32.0]), screen));
        assert_eq!(
            transform.unpoint(transform.point([60.0, 32.0])),
            [60.0, 32.0]
        );
        assert!((transform.rect(rect)[2] - 150.0).abs() < 0.001);
    }
    #[test]
    fn menu_scale_fits_small_windows_and_recovers_after_resize() {
        assert_eq!(MenuTransform::new([1920.0, 1080.0], 0.35).scale, 0.35);
        assert_eq!(MenuTransform::new([1920.0, 1080.0], 2.1).scale, 2.1);
        for viewport in [[120.0, 80.0], [320.0, 240.0], [1920.0, 1080.0]] {
            let t = MenuTransform::new(viewport, 2.0);
            assert!(t.scale.is_finite() && t.scale > 0.0);
            assert!(t.logical_size[0] >= 230.0 && t.logical_size[1] >= 160.0);
        }
        assert_eq!(MenuTransform::new([1920.0, 1080.0], 2.0).scale, 2.0);
    }
    #[test]
    fn clear_binding_requires_an_explicit_action_and_persists() {
        assert_eq!(clear_binding(), "key.keyboard.unknown");
        assert!(Intent::Bind("menu", clear_binding().into()).persists_after_external_close());
        let mut m = Model {
            open: true,
            ..Default::default()
        };
        m.capture = Some("menu");
        m.capture = None; // Escape cancels capture and never means Clear.
        assert!(m.intents.is_empty());
        m.capture = Some("menu");
        m.clear_binding("menu");
        assert_eq!(m.capture, None);
        assert_eq!(
            m.intents.pop_front(),
            Some(Intent::Bind("menu", "key.keyboard.unknown".into()))
        );
        assert_eq!(m.intents.pop_front(), Some(Intent::Flush));
    }
}

/// Explicit unbinding uses the engine's persisted null key, never absence (which
/// restores the registered default). Escape remains capture cancellation.
pub fn clear_binding() -> &'static str {
    "key.keyboard.unknown"
}

#[derive(Clone, Copy, Debug)]
pub struct MenuTransform {
    pub scale: f32,
    pub logical_size: [f32; 2],
}
impl MenuTransform {
    pub fn new(viewport: [f32; 2], requested: f32) -> Self {
        let viewport = viewport.map(|n| if n.is_finite() { n.max(1.0) } else { 1.0 });
        let requested = if requested.is_finite() {
            requested.clamp(catalog::MIN_MENU_SCALE, catalog::MAX_MENU_SCALE)
        } else {
            catalog::DEFAULT_MENU_SCALE
        };
        // Even small windows retain access to reset, close, bindings and filters.
        let scale = requested
            .min(viewport[0] / 320.0)
            .min(viewport[1] / 240.0)
            .max(f32::EPSILON);
        Self {
            scale,
            logical_size: viewport.map(|n| n / scale),
        }
    }
    pub fn point(self, p: [f32; 2]) -> [f32; 2] {
        p.map(|n| n * self.scale)
    }
    pub fn unpoint(self, p: [f32; 2]) -> [f32; 2] {
        p.map(|n| n / self.scale)
    }
    pub fn rect(self, r: [f32; 4]) -> [f32; 4] {
        r.map(|n| n * self.scale)
    }
}
