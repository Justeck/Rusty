// Rusty compact native toolkit menu, Copyright 2026 Justeck.
// SPDX-License-Identifier: Apache-2.0
use super::{Game, Screen};
use crate::game::rusty_ui_controls::Range;
use horizon_client::{
    clipboard::{Clipboard, EditShortcut},
    hud::HudBatch,
    items::ItemStack,
    options_ui::Page,
    ui::{self, EditBox},
};
use horizon_client_render::mesher::rusty::{self, ActivationMode, Config};
use horizon_client_render::text::Text;
use std::{cell::RefCell, collections::BTreeMap, ops::ControlFlow};

type UiResult = ControlFlow<horizon_client::Result<()>>;
const BG: u32 = 0xf0242424;
const ACCENT: u32 = 0xffeeeeee;
const WHITE: u32 = 0xffeeeeee;
const MUTED: u32 = 0xffaaaaaa;
const ERROR: u32 = 0xffffa68d;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Overview,
    Xray,
    Targets,
    Visuals,
    Camera,
    Profiles,
}
impl Tab {
    fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Xray => "Xray",
            Self::Targets => "Targets",
            Self::Visuals => "Visuals",
            Self::Camera => "Camera",
            Self::Profiles => "Profiles",
        }
    }
}
#[derive(Clone, PartialEq)]
enum Action {
    Tab(Tab),
    Close,
    Apply,
    Help(String),
    Toggle(&'static str),
    Activation,
    TargetList,
    Catalog,
    Edit(usize),
    Add(String),
    Held,
    Aimed,
    Remove,
    Preset(&'static str),
    Reset,
    Clear,
    Export,
    Import,
    SaveFile,
    LoadFile,
    Scan,
    Field(usize),
    Slider(&'static str, [f32; 2]),
    Step(&'static str, f32),
}
struct Hit {
    rect: [f32; 4],
    action: Action,
}
struct Field {
    key: &'static str,
    edit: EditBox,
    seen: bool,
}
struct Menu {
    open: bool,
    from_playing: bool,
    previous: Screen,
    tab: Tab,
    cfg: Config,
    fields: Vec<Field>,
    hits: Vec<Hit>,
    scroll: f32,
    content: f32,
    viewport: [f32; 4],
    focus: Option<usize>,
    held_field: bool,
    held_slider: Option<(&'static str, [f32; 2])>,
    keyboard_action: Option<Action>,
    selected: Option<usize>,
    catalog: bool,
    names: Vec<String>,
    held: Option<String>,
    aimed: Option<String>,
    status: String,
    error: bool,
    dirty: bool,
    help: Option<String>,
    confirm: Option<&'static str>,
}
impl Default for Menu {
    fn default() -> Self {
        Self {
            open: false,
            from_playing: false,
            previous: Screen::Playing,
            tab: Tab::Overview,
            cfg: Config::default(),
            fields: Vec::new(),
            hits: Vec::new(),
            scroll: 0.0,
            content: 0.0,
            viewport: [0.0; 4],
            focus: None,
            held_field: false,
            held_slider: None,
            keyboard_action: None,
            selected: None,
            catalog: false,
            names: Vec::new(),
            held: None,
            aimed: None,
            status: String::new(),
            error: false,
            dirty: false,
            help: None,
            confirm: None,
        }
    }
}
thread_local! { static MENU: RefCell<Menu> = RefCell::new(Menu::default()); }

pub(super) fn is_open() -> bool {
    MENU.with(|m| m.borrow().open)
}
pub(super) fn open(game: &mut Game<'_>, from_playing: bool) {
    if !game.native_window() || !game.native_on(rusty::ID) {
        return;
    }
    if from_playing && game.using.is_some() {
        game.native_chat("Finish using the held item before opening Rusty.");
        return;
    }
    let held = game
        .inventory
        .get(game.selected)
        .and_then(Option::as_ref)
        .and_then(|s| game.res.items().get(&s.item))
        .and_then(|i| i.block)
        .map(|id| game.res.facts().get(id).name.to_string());
    let aimed = game.pick().map(|hit| {
        let p = hit.pos;
        game.res
            .facts()
            .get(game.world.block(p[0], p[1], p[2]))
            .name
            .to_string()
    });
    let previous = game.screen;
    let mut names: Vec<String> = game
        .res
        .facts()
        .states
        .iter()
        .map(|s| s.name.to_string())
        .collect();
    names.sort_unstable();
    names.dedup();
    MENU.with(|state| {
        *state.borrow_mut() = Menu {
            open: true,
            from_playing,
            previous,
            cfg: (*rusty::config()).clone(),
            names,
            held,
            aimed,
            status: "Changes save with Apply or Close".into(),
            ..Menu::default()
        };
    });
    if game.screen != Screen::Options {
        game.open_options_at(false, Page::Main);
    }
    super::rusty_control::consume_editor_inputs(game);
    game.keys.clear();
    game.left = false;
    game.right = false;
    game.capture(false);
}
pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    MENU.with(|state| {
        let mut m = state.borrow_mut();
        if m.open
            && (game.screen != Screen::Options
                || !game.native_on(rusty::ID)
                || (m.from_playing && game.player.is_none()))
        {
            m.open = false;
            m.focus = None;
            m.held_field = false;
            m.held_slider = None;
            m.keyboard_action = None;
            m.help = None;
        }
        if !game.focused {
            m.held_field = false;
            m.held_slider = None;
            game.dragging = false;
        }
    });
}
pub(super) fn settings(_game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id == rusty::ID {
        buttons.push("Open Rusty Toolkit".into());
    }
}
pub(super) fn click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id == rusty::ID && button == 0 {
        open(game, false);
    }
}

fn contains(p: (f32, f32), r: [f32; 4]) -> bool {
    ui::hovered(p, r[0], r[1], r[2], r[3])
}
fn fill(game: &Game<'_>, batch: &mut HudBatch, r: [f32; 4], c: u32) {
    game.native_fill(batch, r, c);
}
fn text(game: &Game<'_>, batch: &mut HudBatch, value: &str, x: f32, y: f32, c: u32) {
    game.native_text(batch, value, [x, y], c, false);
}
fn fit(game: &Game<'_>, value: &str, width: f32) -> String {
    if game.native_text_width(value) <= width {
        return value.into();
    }
    let mut out = String::new();
    for c in value.chars() {
        if game.native_text_width(&format!("{out}{c}..")) > width {
            break;
        }
        out.push(c);
    }
    out.push_str("..");
    out
}
fn button(
    m: &mut Menu,
    game: &Game<'_>,
    b: &mut HudBatch,
    rect: [f32; 4],
    label: &str,
    action: Action,
    active: bool,
    accent: bool,
) {
    let hover =
        active && (contains(game.cursor, rect) || m.keyboard_action.as_ref() == Some(&action));
    let label = if accent {
        format!("> {label}")
    } else {
        label.into()
    };
    ui::button(
        b,
        &game.gui.atlas,
        rect[0],
        rect[1],
        rect[2],
        rect[3],
        &Text::literal(fit(game, &label, rect[2] - 8.0)),
        active,
        hover,
    );
    if active {
        horizon_client::widgets::target(rect);
    }
    if active {
        m.hits.push(Hit { rect, action });
    }
}
fn help(m: &mut Menu, game: &Game<'_>, b: &mut HudBatch, x: f32, y: f32, message: &str) {
    button(
        m,
        game,
        b,
        [x, y, 14.0, 16.0],
        "?",
        Action::Help(message.into()),
        true,
        false,
    );
}
fn line(m: &Menu, y: f32) -> f32 {
    m.viewport[1] + y - m.scroll
}
fn visible(m: &Menu, y: f32, h: f32) -> bool {
    y + h > m.viewport[1] && y < m.viewport[1] + m.viewport[3]
}
fn note(m: &Menu, game: &Game<'_>, b: &mut HudBatch, y: &mut f32, value: &str) {
    let yy = line(m, *y);
    if visible(m, yy, 14.0) {
        text(
            game,
            b,
            &fit(game, value, m.viewport[2] - 8.0),
            m.viewport[0] + 3.0,
            yy + 3.0,
            MUTED,
        );
    }
    *y += 16.0;
}
fn row_button(
    m: &mut Menu,
    game: &Game<'_>,
    b: &mut HudBatch,
    y: &mut f32,
    label: &str,
    action: Action,
    active: bool,
) {
    let yy = line(m, *y);
    let rect = [m.viewport[0], yy, m.viewport[2], 18.0];
    if visible(m, yy, 18.0) {
        button(m, game, b, rect, label, action, active, false);
    }
    *y += 21.0;
}
fn toggle(
    m: &mut Menu,
    game: &Game<'_>,
    b: &mut HudBatch,
    y: &mut f32,
    label: &str,
    key: &'static str,
    on: bool,
    message: &str,
) {
    let yy = line(m, *y);
    let x = m.viewport[0];
    let w = m.viewport[2];
    if visible(m, yy, 18.0) {
        button(
            m,
            game,
            b,
            [x, yy, w - 18.0, 18.0],
            &format!("{}  {label}", if on { "ON" } else { "OFF" }),
            Action::Toggle(key),
            true,
            on,
        );
        help(m, game, b, x + w - 14.0, yy + 1.0, message);
    }
    *y += 21.0;
}
fn field(
    m: &mut Menu,
    game: &Game<'_>,
    b: &mut HudBatch,
    y: &mut f32,
    key: &'static str,
    label: &str,
    value: String,
    message: &str,
) {
    if let Some(range) = slider_range(key) {
        let value = value.parse::<f32>().unwrap_or(range.min);
        number_slider(m, game, b, y, key, label, value, range, message);
        return;
    }
    let yy = line(m, *y);
    let x = m.viewport[0];
    let w = m.viewport[2];
    let i = if let Some(i) = m.fields.iter().position(|f| f.key == key) {
        i
    } else {
        let mut edit = EditBox::new(0.0, 0.0, 0.0, 18.0);
        edit.focused = false;
        edit.max_length = 1024;
        edit.set_value(&value);
        m.fields.push(Field {
            key,
            edit,
            seen: false,
        });
        m.fields.len() - 1
    };
    m.fields[i].seen = true;
    let rect = [x, yy + 12.0, w, 18.0];
    {
        let edit = &mut m.fields[i].edit;
        [edit.x, edit.y, edit.width, edit.height] = rect;
    }
    if visible(m, yy, 31.0) {
        text(
            game,
            b,
            &fit(game, label, w - 22.0),
            x + 2.0,
            yy + 1.0,
            MUTED,
        );
        help(m, game, b, x + w - 14.0, yy - 2.0, message);
        if key == "rule_color" {
            if let Ok(color) = u32::from_str_radix(
                m.fields[i]
                    .edit
                    .value
                    .trim()
                    .trim_start_matches('#')
                    .trim_start_matches("0x"),
                16,
            ) {
                fill(game, b, [x + w - 32.0, yy, 12.0, 8.0], color);
            }
        }
        let focused = m.focus == Some(i);
        let f = &mut m.fields[i];
        [f.edit.x, f.edit.y, f.edit.width, f.edit.height] = rect;
        f.edit.focused = focused;
        f.edit.draw(b, &game.gui.atlas, None);
        m.hits.push(Hit {
            rect,
            action: Action::Field(i),
        });
    }
    *y += 35.0;
}
fn slider_range(key: &str) -> Option<Range> {
    let (min, max, step) = match key {
        "texture_visibility" => (0.0, 100.0, 1.0),
        "context_radius" => (1.0, 48.0, 1.0),
        "context_opacity" => (0.0, 255.0, 1.0),
        "scan_radius" | "rule_radius" => (1.0, 128.0, 1.0),
        "scan_interval" => (1.0, 60.0, 1.0),
        "max_targets" => (1.0, 4096.0, 1.0),
        "tracer_limit" => (0.0, 12.0, 1.0),
        "line_width" => (0.5, 4.0, 0.1),
        "zoom_factor" => (1.0, 16.0, 0.25),
        "freecam_speed" => (0.05, 8.0, 0.05),
        "rule_min_y" | "rule_max_y" => (-2048.0, 2047.0, 1.0),
        _ => return None,
    };
    Some(Range { min, max, step })
}
fn slider_current(m: &Menu, key: &str) -> f32 {
    match key {
        "texture_visibility" => m.cfg.texture_visibility(),
        "context_radius" => m.cfg.context_radius as f32,
        "context_opacity" => m.cfg.context_opacity as f32,
        "scan_radius" => m.cfg.scan_radius as f32,
        "scan_interval" => m.cfg.scan_interval as f32,
        "max_targets" => m.cfg.max_targets as f32,
        "tracer_limit" => m.cfg.tracer_limit as f32,
        "line_width" => m.cfg.line_width,
        "zoom_factor" => m.cfg.zoom_factor,
        "freecam_speed" => m.cfg.freecam_speed as f32,
        _ => m
            .selected
            .and_then(|i| m.cfg.rules.get(i))
            .map_or(0.0, |r| match key {
                "rule_radius" => r.radius as f32,
                "rule_min_y" => r.min_y as f32,
                "rule_max_y" => r.max_y as f32,
                _ => 0.0,
            }),
    }
}
fn set_slider(m: &mut Menu, key: &str, value: f32) -> Result<(), String> {
    match key {
        "texture_visibility" => m.cfg.set_texture_visibility(value)?,
        "context_radius" => m.cfg.context_radius = value as u32,
        "context_opacity" => m.cfg.context_opacity = value as u8,
        "scan_radius" => m.cfg.scan_radius = value as u32,
        "scan_interval" => m.cfg.scan_interval = value as u32,
        "max_targets" => m.cfg.max_targets = value as u32,
        "tracer_limit" => m.cfg.tracer_limit = value as u32,
        "line_width" => m.cfg.line_width = value,
        "zoom_factor" => m.cfg.zoom_factor = value,
        "freecam_speed" => m.cfg.freecam_speed = f64::from(value),
        _ => {
            if let Some(r) = m.selected.and_then(|i| m.cfg.rules.get_mut(i)) {
                match key {
                    "rule_radius" => r.radius = value as u32,
                    "rule_min_y" => r.min_y = (value as i32).min(r.max_y),
                    "rule_max_y" => r.max_y = (value as i32).max(r.min_y),
                    _ => {}
                }
            }
        }
    }
    m.dirty = true;
    Ok(())
}
fn number_slider(
    m: &mut Menu,
    g: &Game<'_>,
    b: &mut HudBatch,
    y: &mut f32,
    key: &'static str,
    label: &str,
    value: f32,
    range: Range,
    message: &str,
) {
    let yy = line(m, *y);
    let [x, _, w, _] = m.viewport;
    let texture = key == "texture_visibility";
    if visible(m, yy, 34.0) {
        text(g, b, &fit(g, label, w - 22.0), x + 2.0, yy + 1.0, MUTED);
        help(m, g, b, x + w - 14.0, yy - 2.0, message);
        let track = [x + 22.0, w - 44.0];
        let rect = [track[0], yy + 12.0, track[1], 20.0];
        let keyboard_focus = matches!(&m.keyboard_action,
            Some(Action::Slider(k, _)) | Some(Action::Step(k, _)) if *k == key);
        let hover = contains(g.cursor, rect) || keyboard_focus;
        let shown = if texture {
            if value >= 100.0 {
                "Normal: 100%".into()
            } else if value <= 0.0 {
                "Outline: 0%".into()
            } else {
                format!("{value:.0}%")
            }
        } else if range.step < 1.0 {
            format!("{value:.2}")
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        } else {
            format!("{value:.0}")
        };
        ui::slider(
            b,
            &g.gui.atlas,
            rect[0],
            rect[1],
            rect[2],
            &Text::literal(format!("{label}: {shown}")),
            range.fraction(value),
            hover,
            true,
            m.held_slider.is_some_and(|(held, _)| held == key),
        );
        horizon_client::widgets::widget_cursor(hover, true);
        m.hits.push(Hit {
            rect,
            action: Action::Slider(key, track),
        });
        button(
            m,
            g,
            b,
            [x, yy + 12.0, 20.0, 20.0],
            "-",
            Action::Step(key, -1.0),
            value > range.min,
            false,
        );
        button(
            m,
            g,
            b,
            [x + w - 20.0, yy + 12.0, 20.0, 20.0],
            "+",
            Action::Step(key, 1.0),
            value < range.max,
            false,
        );
    }
    *y += 36.0;
    if texture {
        let yy = line(m, *y);
        if visible(m, yy, 10.0) {
            text(g, b, "Outline", x + 22.0, yy, MUTED);
            text(
                g,
                b,
                "Normal",
                x + w - 22.0 - g.native_text_width("Normal"),
                yy,
                MUTED,
            );
        }
        *y += 12.0;
    }
}
fn icon(game: &Game<'_>, b: &mut HudBatch, id: &str, x: f32, y: f32, color: u32) {
    if game.res.items().get(id).is_some() {
        let stack = ItemStack {
            item: id.into(),
            count: 1,
            damage: 0,
            extra: None,
        };
        game.draw_item_model(b, &stack, x, y);
    } else {
        fill(game, b, [x + 3.0, y + 3.0, 10.0, 10.0], color);
    }
}

pub(super) fn draw(game: &mut Game<'_>, batch: &mut HudBatch) -> ControlFlow<()> {
    MENU.with(|state| {
        let mut m = state.borrow_mut();
        if !m.open || game.screen != Screen::Options {
            return ControlFlow::Continue(());
        }
        m.hits.clear();
        for f in &mut m.fields {
            f.seen = false;
        }
        let w = (batch.width - 12.0).clamp(220.0, 470.0);
        let h = (batch.height - 12.0).clamp(170.0, 326.0);
        let x = ((batch.width - w) / 2.0).floor();
        let y = ((batch.height - h) / 2.0).floor();
        let full_screen = [0.0, 0.0, batch.width, batch.height];
        ui::menu_background(
            batch,
            &game.gui.atlas,
            full_screen[0],
            full_screen[1],
            full_screen[2],
            full_screen[3],
            game.player.is_some(),
        );
        fill(
            game,
            batch,
            [x - 1.0, y - 1.0, w + 2.0, h + 2.0],
            0xff777777,
        );
        fill(game, batch, [x, y, w, h], BG);
        fill(game, batch, [x + 1.0, y + 1.0, w - 2.0, 1.0], 0xff999999);
        text(game, batch, "Rusty", x + 10.0, y + 10.0, ACCENT);
        text(game, batch, "CLIENT TOOLKIT", x + 62.0, y + 11.0, MUTED);
        button(
            &mut m,
            game,
            batch,
            [x + w - 22.0, y + 6.0, 16.0, 18.0],
            "X",
            Action::Close,
            true,
            false,
        );
        let tabs = [
            Tab::Overview,
            Tab::Xray,
            Tab::Targets,
            Tab::Visuals,
            Tab::Camera,
            Tab::Profiles,
        ];
        let nav_w = if w < 350.0 { 58.0 } else { 76.0 };
        for (i, tab) in tabs.into_iter().enumerate() {
            let chosen = m.tab == tab;
            button(
                &mut m,
                game,
                batch,
                [x + 6.0, y + 32.0 + i as f32 * 23.0, nav_w, 20.0],
                tab.title(),
                Action::Tab(tab),
                true,
                chosen,
            );
        }
        text(game, batch, rusty::VERSION, x + 9.0, y + h - 40.0, MUTED);
        let vx = x + nav_w + 13.0;
        let vy = y + 33.0;
        let vw = w - nav_w - 20.0;
        let vh = h - 88.0;
        m.viewport = [vx, vy, vw, vh];
        let start = batch.vertices.len();
        let hit_start = m.hits.len();
        let target_start = horizon_client::widgets::targets_mark();
        let mut pos = 0.0;
        match m.tab {
            Tab::Overview => overview(&mut m, game, batch, &mut pos),
            Tab::Xray => xray(&mut m, game, batch, &mut pos),
            Tab::Targets => targets(&mut m, game, batch, &mut pos),
            Tab::Visuals => visuals(&mut m, game, batch, &mut pos),
            Tab::Camera => camera(&mut m, game, batch, &mut pos),
            Tab::Profiles => profiles(&mut m, game, batch, &mut pos),
        }
        batch.clip_since(start, vx, vy, vx + vw, vy + vh);
        // Input clips exactly where pixels clip; hidden rows cannot be clicked.
        for hit in m.hits.iter_mut().skip(hit_start) {
            let bottom = (hit.rect[1] + hit.rect[3]).min(vy + vh);
            hit.rect[1] = hit.rect[1].max(vy);
            hit.rect[3] = (bottom - hit.rect[1]).max(0.0);
        }
        let mut index = 0;
        m.hits.retain(|hit| {
            let keep = index < hit_start || hit.rect[3] > 0.0;
            index += 1;
            keep
        });
        horizon_client::widgets::clip_targets(target_start, vy, vy + vh);
        m.content = pos;
        m.scroll = m.scroll.clamp(0.0, (pos - vh).max(0.0));
        if pos > vh {
            let thumb = (vh * vh / pos).max(12.0);
            let at = m.scroll / (pos - vh) * (vh - thumb);
            fill(
                game,
                batch,
                [vx + vw - 2.0, vy + at, 2.0, thumb],
                0xffaaaaaa,
            );
        }
        let status_color = if m.error { ERROR } else { MUTED };
        text(
            game,
            batch,
            &fit(game, &m.status, w - 16.0),
            x + 8.0,
            y + h - 33.0,
            status_color,
        );
        let apply_label = if m.dirty {
            "Apply changes *"
        } else {
            "Apply changes"
        };
        button(
            &mut m,
            game,
            batch,
            [vx, y + h - 21.0, (vw - 5.0) * 0.62, 17.0],
            apply_label,
            Action::Apply,
            true,
            true,
        );
        button(
            &mut m,
            game,
            batch,
            [
                vx + (vw - 5.0) * 0.62 + 5.0,
                y + h - 21.0,
                (vw - 5.0) * 0.38,
                17.0,
            ],
            "Close",
            Action::Close,
            true,
            false,
        );
        if let Some(message) = m.help.clone() {
            let pw = (w - 28.0).min(330.0);
            let mut lines = Vec::<String>::new();
            let mut current = String::new();
            for word in message.split_whitespace() {
                let next = if current.is_empty() {
                    word.to_string()
                } else {
                    format!("{current} {word}")
                };
                if game.native_text_width(&next) > pw - 16.0 && !current.is_empty() {
                    lines.push(current);
                    current = word.into();
                } else {
                    current = next;
                }
            }
            if !current.is_empty() {
                lines.push(current);
            }
            let ph = (lines.len() as f32 * 11.0 + 28.0).min(h - 16.0);
            let px = x + (w - pw) / 2.0;
            let py = y + (h - ph) / 2.0;
            fill(game, batch, [x, y, w, h], 0xc0101010);
            fill(game, batch, [px, py, pw, ph], 0xff363636);
            text(game, batch, "QUICK HELP", px + 8.0, py + 7.0, ACCENT);
            for (i, line) in lines.iter().enumerate() {
                if 23.0 + i as f32 * 11.0 < ph - 3.0 {
                    text(
                        game,
                        batch,
                        line,
                        px + 8.0,
                        py + 23.0 + i as f32 * 11.0,
                        WHITE,
                    );
                }
            }
        }
        ControlFlow::Break(())
    })
}

fn overview(m: &mut Menu, g: &Game<'_>, b: &mut HudBatch, y: &mut f32) {
    let stats = crate::game::rusty_visuals::stats();
    note(m, g, b, y, "Your world, a little clearer.");
    note(
        m,
        g,
        b,
        y,
        &format!(
            "{} targets / {} enabled",
            m.cfg.rules.len(),
            m.cfg.rules.iter().filter(|r| r.enabled).count()
        ),
    );
    note(
        m,
        g,
        b,
        y,
        &format!(
            "Xray is {}",
            if rusty::active() { "active" } else { "off" }
        ),
    );
    note(
        m,
        g,
        b,
        y,
        &format!(
            "{} matches / {} pending{}",
            stats.total,
            stats.pending,
            if stats.truncated { " / capped" } else { "" }
        ),
    );
    row_button(
        m,
        g,
        b,
        y,
        "Scan loaded blocks now",
        Action::Scan,
        g.player.is_some(),
    );
    row_button(
        m,
        g,
        b,
        y,
        "Activation and terrain",
        Action::Tab(Tab::Xray),
        true,
    );
    row_button(
        m,
        g,
        b,
        y,
        "Find and edit target blocks",
        Action::Tab(Tab::Targets),
        true,
    );
    row_button(
        m,
        g,
        b,
        y,
        "Camera and client utilities",
        Action::Tab(Tab::Camera),
        true,
    );
    note(m, g, b, y, "Click ? for a short explanation.");
    note(m, g, b, y, "Use your mod keybinds to activate.");
    note(m, g, b, y, "Only received world data is available.");
}
fn xray(m: &mut Menu, g: &Game<'_>, b: &mut HudBatch, y: &mut f32) {
    let c = m.cfg.clone();
    let label = format!(
        "Activation: {}",
        if m.cfg.activation == ActivationMode::Toggle {
            "Toggle"
        } else {
            "Hold"
        }
    );
    row_button(m, g, b, y, &label, Action::Activation, true);
    toggle(
        m,
        g,
        b,
        y,
        "Terrain context",
        "context",
        c.context_outlines,
        "Shows the original textures around nearby block edges while Xray is active, so you can recognize floors, walls and steps. On by default.",
    );
    field(
        m,
        g,
        b,
        y,
        "context_radius",
        "Terrain radius",
        c.context_radius.to_string(),
        "Radius in blocks for the nearby terrain context. Smaller values reduce visual clutter.",
    );
    field(
        m,
        g,
        b,
        y,
        "context_opacity",
        "Terrain opacity (0-255)",
        c.context_opacity.to_string(),
        "Opacity of textured terrain borders: 0 hides them, 255 keeps the original texture fully visible.",
    );
    field(
        m,
        g,
        b,
        y,
        "texture_visibility",
        "Block textures",
        c.texture_visibility().to_string(),
        "Outline keeps a thin, nonzero edge of the original block texture. The default is 25% of the face. Increase toward Normal to fill the block faces; 100% restores the normal game terrain, lighting and distant rendering. Apply or Close saves the setting. The Terrain context switch must be on.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Block finder",
        "finder",
        c.finder_enabled,
        "Scan blocks already received by this client, using each target's radius and Y bounds. This cannot reveal chunks the server has not sent.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Automatic scans",
        "auto_scan",
        c.auto_scan,
        "Refresh the target scan automatically. The interval controls how often it updates.",
    );
    field(
        m,
        g,
        b,
        y,
        "scan_radius",
        "Scan radius",
        c.scan_radius.to_string(),
        "Global scan distance in blocks. Per-target radii can restrict this further.",
    );
    field(
        m,
        g,
        b,
        y,
        "scan_interval",
        "Scan interval (ticks)",
        c.scan_interval.to_string(),
        "Delay between automatic scans. Twenty game ticks is approximately one second.",
    );
    field(
        m,
        g,
        b,
        y,
        "max_targets",
        "Maximum target results",
        c.max_targets.to_string(),
        "Caps retained matches to keep scanning and rendering bounded.",
    );
}
fn targets(m: &mut Menu, g: &Game<'_>, b: &mut HudBatch, y: &mut f32) {
    if let Some(index) = m.selected {
        if index >= m.cfg.rules.len() {
            m.selected = None;
            return;
        }
        row_button(
            m,
            g,
            b,
            y,
            "< All selected targets",
            Action::TargetList,
            true,
        );
        let r = m.cfg.rules[index].clone();
        field(
            m,
            g,
            b,
            y,
            "rule_id",
            "Block identifier",
            r.id.clone(),
            "Exact registered block identifier, including namespace. Use the catalogue to discover blocks.",
        );
        field(
            m,
            g,
            b,
            y,
            "rule_label",
            "Display label",
            r.label.clone(),
            "Optional name for this target. An empty label uses the block's name.",
        );
        toggle(
            m,
            g,
            b,
            y,
            "Target enabled",
            "rule_enabled",
            r.enabled,
            "Include this block in Xray selection and scanning. Disabling preserves its settings.",
        );
        field(
            m,
            g,
            b,
            y,
            "rule_color",
            "Color (AARRGGBB)",
            format!("{:08X}", r.color),
            "Eight hexadecimal digits: alpha, red, green and blue. Example FF60DED0 is opaque teal.",
        );
        field(
            m,
            g,
            b,
            y,
            "rule_radius",
            "Scan radius in blocks",
            r.radius.to_string(),
            "Horizontal radius for scanning and markers around your player. Selected block models use the Y limits; model visibility is not restricted by this radius.",
        );
        field(
            m,
            g,
            b,
            y,
            "rule_min_y",
            "Minimum Y",
            r.min_y.to_string(),
            "Lowest included world height for this block, inclusive.",
        );
        field(
            m,
            g,
            b,
            y,
            "rule_max_y",
            "Maximum Y",
            r.max_y.to_string(),
            "Highest included world height for this block, inclusive. Must not be below minimum Y.",
        );
        let states = r
            .states
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ");
        field(
            m,
            g,
            b,
            y,
            "rule_states",
            "States (key=value, ...)",
            states,
            "Optional exact block-state filters separated by commas, such as facing=north, lit=true. Leave empty to match every state.",
        );
        field(
            m,
            g,
            b,
            y,
            "rule_group",
            "Group",
            r.group.clone(),
            "A label to organize related targets. This does not change which blocks match.",
        );
        toggle(
            m,
            g,
            b,
            y,
            "Outline",
            "rule_outline",
            r.outline,
            "Show an outline around results for this target when global block outlines are enabled.",
        );
        toggle(
            m,
            g,
            b,
            y,
            "Tracer",
            "rule_tracers",
            r.tracers,
            "Allow tracer lines for this target when global tracers are enabled.",
        );
        row_button(m, g, b, y, "Remove this target", Action::Remove, true);
        return;
    }
    let catalog = m.catalog;
    let held = m.held.is_some();
    let aimed = m.aimed.is_some();
    row_button(
        m,
        g,
        b,
        y,
        if catalog {
            "< Selected targets"
        } else {
            "+ Add from block catalogue"
        },
        if catalog {
            Action::TargetList
        } else {
            Action::Catalog
        },
        true,
    );
    if m.catalog {
        row_button(m, g, b, y, "+ Add held block", Action::Held, held);
        row_button(m, g, b, y, "+ Add aimed block", Action::Aimed, aimed);
    }
    field(
        m,
        g,
        b,
        y,
        "search",
        if catalog {
            "Search every registered block"
        } else {
            "Search selected targets"
        },
        String::new(),
        "Search by block name, namespace, identifier or your custom label. Results update as you type.",
    );
    let query = m
        .fields
        .iter()
        .find(|f| f.key == "search")
        .map(|f| f.edit.value.to_lowercase())
        .unwrap_or_default();
    let mut count = 0usize;
    if m.catalog {
        let names = m.names.clone();
        for id in names {
            let label = rusty::friendly_name(&id);
            if !query.is_empty() && !format!("{id} {label}").to_lowercase().contains(&query) {
                continue;
            }
            count += 1;
            let yy = line(m, *y);
            let added = m.cfg.rules.iter().any(|r| r.id == id);
            if visible(m, yy, 20.0) {
                let r = [m.viewport[0], yy, m.viewport[2], 20.0];
                button(
                    m,
                    g,
                    b,
                    r,
                    &format!("    {}{}", if added { "[+] " } else { "+ " }, label),
                    Action::Add(id.clone()),
                    !added,
                    false,
                );
                icon(g, b, &id, r[0] + 2.0, yy + 2.0, ACCENT);
            }
            *y += 22.0;
        }
    } else {
        let rules = m.cfg.rules.clone();
        for (i, r) in rules.iter().enumerate() {
            let label = r.display_name();
            if !query.is_empty()
                && !format!("{} {label} {}", r.id, r.group)
                    .to_lowercase()
                    .contains(&query)
            {
                continue;
            }
            count += 1;
            let yy = line(m, *y);
            if visible(m, yy, 20.0) {
                let rect = [m.viewport[0], yy, m.viewport[2], 20.0];
                button(
                    m,
                    g,
                    b,
                    rect,
                    &format!("    {}{}", if r.enabled { "" } else { "(off) " }, label),
                    Action::Edit(i),
                    true,
                    false,
                );
                icon(g, b, &r.id, rect[0] + 2.0, yy + 2.0, r.color);
            }
            *y += 22.0;
        }
    }
    if count == 0 {
        note(m, g, b, y, "No matching blocks.");
    }
}
fn visuals(m: &mut Menu, g: &Game<'_>, b: &mut HudBatch, y: &mut f32) {
    let c = m.cfg.clone();
    toggle(
        m,
        g,
        b,
        y,
        "Block outlines",
        "outlines",
        c.block_outlines,
        "Draw colored boxes around scanned target matches. Each target can opt out.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Tracer lines",
        "tracers",
        c.tracers,
        "Connect your view to nearby scanned target blocks. Each target can opt out.",
    );
    field(
        m,
        g,
        b,
        y,
        "tracer_limit",
        "Maximum tracer lines",
        c.tracer_limit.to_string(),
        "Limit the number of visible tracer lines so nearby matches remain readable.",
    );
    field(
        m,
        g,
        b,
        y,
        "line_width",
        "Line width",
        c.line_width.to_string(),
        "Outline and tracer width in pixels. Thin lines keep the scene readable.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Status HUD",
        "hud",
        c.hud,
        "Show Rusty's current mode and scan status in the gameplay HUD.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Per-target counts",
        "hud_counts",
        c.hud_counts,
        "Include target result counts in the HUD. Counts represent loaded client data.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Entity ESP",
        "entity_esp",
        c.entity_esp,
        "Mark selected categories of entities that the server has already sent to your client.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "ESP players",
        "esp_players",
        c.esp_players,
        "Include other players in entity ESP.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "ESP mobs",
        "esp_mobs",
        c.esp_mobs,
        "Include living non-player entities in entity ESP.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "ESP dropped items",
        "esp_items",
        c.esp_items,
        "Include dropped item entities in entity ESP.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Coordinates HUD",
        "coords_hud",
        c.coords_hud,
        "Show your player coordinates in the gameplay HUD.",
    );
}
fn camera(m: &mut Menu, g: &Game<'_>, b: &mut HudBatch, y: &mut f32) {
    let c = m.cfg.clone();
    note(m, g, b, y, "Freecam uses its own mod keybind.");
    field(
        m,
        g,
        b,
        y,
        "freecam_speed",
        "Freecam speed",
        c.freecam_speed.to_string(),
        "Movement multiplier for the detached camera. The player remains at the original position.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Freecam noclip",
        "noclip",
        c.freecam_noclip,
        "Allow the detached camera to move through blocks. It does not move the server-side player.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Fullbright",
        "fullbright",
        c.fullbright,
        "Brighten the local rendered scene for visibility in dark areas.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Clear fog",
        "no_fog",
        c.no_fog,
        "Remove local distance fog. This cannot create missing chunk data.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Clear weather view",
        "weather",
        c.clear_weather,
        "Hide precipitation in your view without changing the world's weather.",
    );
    field(
        m,
        g,
        b,
        y,
        "zoom_factor",
        "Zoom multiplier",
        c.zoom_factor.to_string(),
        "Magnification used by the zoom mod keybind. A value of 4 means four-times magnification.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Auto sprint",
        "auto_sprint",
        c.auto_sprint,
        "Automatically request sprint while moving forward, subject to the game's normal sprint conditions.",
    );
    toggle(
        m,
        g,
        b,
        y,
        "Stable hurt camera",
        "no_hurt",
        c.no_hurt_camera,
        "Suppress the local camera shake caused by taking damage.",
    );
}
fn profiles(m: &mut Menu, g: &Game<'_>, b: &mut HudBatch, y: &mut f32) {
    let reset = m.confirm == Some("reset");
    let clear = m.confirm == Some("clear");
    note(m, g, b, y, "Portable, versioned JSON settings.");
    row_button(
        m,
        g,
        b,
        y,
        "Export settings to clipboard",
        Action::Export,
        true,
    );
    row_button(
        m,
        g,
        b,
        y,
        "Import settings from clipboard",
        Action::Import,
        true,
    );
    field(
        m,
        g,
        b,
        y,
        "profile_name",
        "Preset name",
        "my-preset".into(),
        "A local preset filename, using letters, numbers, dash and underscore. Files stay inside Rusty's own data folder.",
    );
    row_button(m, g, b, y, "Save named preset", Action::SaveFile, true);
    row_button(m, g, b, y, "Load named preset", Action::LoadFile, true);
    row_button(
        m,
        g,
        b,
        y,
        "Add preset: all ores",
        Action::Preset("ores"),
        true,
    );
    row_button(
        m,
        g,
        b,
        y,
        "Add preset: storage blocks",
        Action::Preset("storage"),
        true,
    );
    row_button(m, g, b, y, "Add preset: lava", Action::Preset("lava"), true);
    row_button(
        m,
        g,
        b,
        y,
        if reset {
            "Confirm: reset all settings?"
        } else {
            "Reset all settings to defaults"
        },
        Action::Reset,
        true,
    );
    row_button(
        m,
        g,
        b,
        y,
        if clear {
            "Confirm: clear every target?"
        } else {
            "Clear every selected target"
        },
        Action::Clear,
        true,
    );
}

fn flush(m: &mut Menu) -> Result<(), String> {
    for f in &m.fields {
        let v = f.edit.value.trim();
        match f.key {
            key if key.starts_with("rule_") => {
                let Some(rule) = m.selected.and_then(|i| m.cfg.rules.get_mut(i)) else {
                    continue;
                };
                match key {
                    "rule_id" => rule.id = v.into(),
                    "rule_label" => rule.label = f.edit.value.clone(),
                    "rule_color" => {
                        rule.color = u32::from_str_radix(
                            v.trim_start_matches('#').trim_start_matches("0x"),
                            16,
                        )
                        .map_err(|_| "Color must be hexadecimal AARRGGBB".to_string())?
                    }
                    "rule_group" => rule.group = f.edit.value.clone(),
                    "rule_states" => {
                        let mut states = BTreeMap::new();
                        for part in v.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                            let (key, value) = part
                                .split_once('=')
                                .ok_or("States must use key=value, separated by commas")?;
                            let (key, value) = (key.trim(), value.trim());
                            if key.is_empty() || value.is_empty() {
                                return Err("Each state needs a property and a value".into());
                            }
                            if states.insert(key.into(), value.into()).is_some() {
                                return Err("A state property was entered twice".into());
                            }
                        }
                        rule.states = states;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    m.cfg.clone().validated()?;
    Ok(())
}
fn apply(m: &mut Menu, g: &mut Game<'_>) -> Result<(), String> {
    flush(m)?;
    crate::game::rusty_control::apply_config(g, m.cfg.clone())?;
    m.cfg = (*rusty::config()).clone();
    m.dirty = false;
    m.error = false;
    m.status = "Settings saved".into();
    Ok(())
}
fn changed_page(m: &mut Menu) {
    m.fields.clear();
    m.focus = None;
    m.held_field = false;
    m.held_slider = None;
    m.keyboard_action = None;
    m.scroll = 0.0;
    m.help = None;
    m.confirm = None;
}
fn close(m: &mut Menu, g: &mut Game<'_>) -> Result<(), String> {
    apply(m, g)?;
    super::rusty_control::consume_editor_inputs(g);
    m.open = false;
    m.focus = None;
    m.held_field = false;
    m.held_slider = None;
    m.keyboard_action = None;
    m.help = None;
    g.dragging = false;
    if m.from_playing {
        g.screen = Screen::Playing;
        g.keys.clear();
        g.capture(true);
    } else {
        g.screen = m.previous;
    }
    Ok(())
}
fn toggle_key(m: &mut Menu, key: &str) {
    macro_rules! flip {
        ($v:expr) => {
            $v = !$v
        };
    }
    match key {
        "context" => flip!(m.cfg.context_outlines),
        "finder" => flip!(m.cfg.finder_enabled),
        "auto_scan" => flip!(m.cfg.auto_scan),
        "outlines" => flip!(m.cfg.block_outlines),
        "tracers" => flip!(m.cfg.tracers),
        "hud" => flip!(m.cfg.hud),
        "hud_counts" => flip!(m.cfg.hud_counts),
        "entity_esp" => flip!(m.cfg.entity_esp),
        "esp_players" => flip!(m.cfg.esp_players),
        "esp_mobs" => flip!(m.cfg.esp_mobs),
        "esp_items" => flip!(m.cfg.esp_items),
        "coords_hud" => flip!(m.cfg.coords_hud),
        "noclip" => flip!(m.cfg.freecam_noclip),
        "fullbright" => flip!(m.cfg.fullbright),
        "no_fog" => flip!(m.cfg.no_fog),
        "weather" => flip!(m.cfg.clear_weather),
        "auto_sprint" => flip!(m.cfg.auto_sprint),
        "no_hurt" => flip!(m.cfg.no_hurt_camera),
        key if key.starts_with("rule_") => {
            if let Some(r) = m.selected.and_then(|i| m.cfg.rules.get_mut(i)) {
                match key {
                    "rule_enabled" => flip!(r.enabled),
                    "rule_outline" => flip!(r.outline),
                    "rule_tracers" => flip!(r.tracers),
                    _ => {}
                }
            }
        }
        _ => {}
    }
    m.dirty = true;
}
fn preset_name(m: &Menu) -> Result<String, String> {
    let name = m
        .fields
        .iter()
        .find(|f| f.key == "profile_name")
        .map(|f| f.edit.value.trim())
        .unwrap_or("my-preset");
    if name.is_empty()
        || name.len() > 64
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("Preset name: use 1-64 letters, numbers, dash or underscore".into());
    }
    Ok(format!("{name}.json"))
}
fn action(m: &mut Menu, g: &mut Game<'_>, action: Action) -> Result<(), String> {
    m.keyboard_action = Some(action.clone());
    match action {
        Action::Help(message) => {
            m.help = Some(message);
            return Ok(());
        }
        Action::Field(i) => {
            m.focus = Some(i);
            m.held_field = true;
            m.held_slider = None;
            g.dragging = true;
            let shift = g.has_shift_down();
            let f = &mut m.fields[i];
            f.edit.focused = true;
            f.edit.click(&g.gui.atlas, g.cursor.0, shift, false);
            f.edit.reset_blink();
            return Ok(());
        }
        _ => {}
    }
    if !matches!(
        action,
        Action::Import | Action::LoadFile | Action::Reset | Action::Clear
    ) {
        flush(m)?;
    }
    m.focus = None;
    m.held_field = false;
    m.held_slider = None;
    match action {
        Action::Slider(key, track) => {
            let range = slider_range(key).ok_or("Unknown slider")?;
            set_slider(
                m,
                key,
                range.value(ui::slider_value(track[0], track[1], g.cursor.0)),
            )?;
            m.held_slider = Some((key, track));
            g.dragging = true;
        }
        Action::Step(key, steps) => {
            let range = slider_range(key).ok_or("Unknown slider")?;
            let next = range.nudge(slider_current(m, key), steps);
            set_slider(m, key, next)?;
        }
        Action::Close => return close(m, g),
        Action::Apply => return apply(m, g),
        Action::Scan => {
            m.cfg.finder_enabled = true;
            apply(m, g)?;
            crate::game::rusty_visuals::request_scan();
            m.status = "Scan requested for loaded blocks".into();
        }
        Action::Tab(tab) => {
            m.tab = tab;
            changed_page(m);
        }
        Action::Activation => {
            m.cfg.activation = if m.cfg.activation == ActivationMode::Toggle {
                ActivationMode::Hold
            } else {
                ActivationMode::Toggle
            };
            m.dirty = true;
        }
        Action::Toggle(key) => toggle_key(m, key),
        Action::TargetList => {
            m.selected = None;
            m.catalog = false;
            changed_page(m);
        }
        Action::Catalog => {
            m.selected = None;
            m.catalog = true;
            changed_page(m);
        }
        Action::Edit(i) => {
            m.selected = Some(i);
            changed_page(m);
        }
        Action::Add(id) => {
            if m.cfg.add_block(&id) {
                m.dirty = true;
                m.status = format!("Added {}", rusty::friendly_name(&id));
            } else {
                m.status = "That block is already selected".into();
            }
        }
        Action::Held | Action::Aimed => {
            let id = if matches!(action, Action::Held) {
                m.held.clone()
            } else {
                m.aimed.clone()
            }
            .ok_or("No block was available when this menu opened")?;
            if m.cfg.add_block(&id) {
                m.dirty = true;
                m.status = format!("Added {}", rusty::friendly_name(&id));
            } else {
                m.status = "That block is already selected".into();
            }
        }
        Action::Remove => {
            if let Some(i) = m.selected.take() {
                if i < m.cfg.rules.len() {
                    m.cfg.rules.remove(i);
                    m.dirty = true;
                }
            }
            changed_page(m);
        }
        Action::Preset(kind) => {
            let names = m.names.clone();
            let mut added = 0;
            for id in names {
                let selected = match kind {
                    "ores" => id.ends_with("_ore") || id == "minecraft:ancient_debris",
                    "storage" => [
                        "chest",
                        "trapped_chest",
                        "ender_chest",
                        "barrel",
                        "shulker_box",
                        "hopper",
                        "dispenser",
                        "dropper",
                        "furnace",
                        "blast_furnace",
                        "smoker",
                    ]
                    .iter()
                    .any(|s| {
                        id == format!("minecraft:{s}")
                            || (*s == "shulker_box" && id.ends_with("_shulker_box"))
                    }),
                    "lava" => id == "minecraft:lava",
                    _ => false,
                };
                if selected && m.cfg.add_block(&id) {
                    added += 1;
                }
            }
            m.dirty |= added > 0;
            m.status = format!("Added {added} targets from {kind} preset");
        }
        Action::Reset => {
            if m.confirm == Some("reset") {
                m.cfg = Config {
                    client: m.cfg.client.clone(),
                    ..Config::default()
                };
                m.selected = None;
                changed_page(m);
                m.dirty = true;
                m.status = "Defaults restored; Apply to save".into();
            } else {
                m.confirm = Some("reset");
                m.status = "Click Reset again to confirm".into();
            }
        }
        Action::Clear => {
            if m.confirm == Some("clear") {
                m.cfg.rules.clear();
                m.selected = None;
                changed_page(m);
                m.dirty = true;
                m.status = "Target list cleared; Apply to save".into();
            } else {
                m.confirm = Some("clear");
                m.status = "Click Clear again to confirm".into();
            }
        }
        Action::Export => {
            g.clipboard.set(&m.cfg.to_json()?);
            m.status = "Settings copied to clipboard".into();
        }
        Action::Import => {
            let c = m.cfg.import_profile(&g.clipboard.get())?;
            crate::game::rusty_control::apply_config(g, c.clone())?;
            m.cfg = c;
            m.selected = None;
            changed_page(m);
            m.dirty = false;
            m.status = "Clipboard settings imported and saved".into();
        }
        Action::SaveFile => {
            let path = g
                .native_data_dir(rusty::ID)
                .join("presets")
                .join(preset_name(m)?);
            std::fs::create_dir_all(path.parent().ok_or("Invalid preset folder")?)
                .map_err(|e| e.to_string())?;
            std::fs::write(&path, m.cfg.to_json()?).map_err(|e| e.to_string())?;
            m.status = format!(
                "Preset saved: {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
        }
        Action::LoadFile => {
            let path = g
                .native_data_dir(rusty::ID)
                .join("presets")
                .join(preset_name(m)?);
            let file =
                std::fs::File::open(&path).map_err(|e| format!("Cannot read preset: {e}"))?;
            let c = m.cfg.import_profile_reader(file)?;
            crate::game::rusty_control::apply_config(g, c.clone())?;
            m.cfg = c;
            m.selected = None;
            changed_page(m);
            m.dirty = false;
            m.status = "Named preset imported and saved".into();
        }
        Action::Help(_) | Action::Field(_) => {}
    }
    Ok(())
}
fn outcome(m: &mut Menu, result: Result<(), String>) {
    if let Err(error) = result {
        m.error = true;
        m.status = error;
    } else {
        m.error = false;
    }
}
pub(super) fn mouse(g: &mut Game<'_>, pressed: bool, right: bool) -> UiResult {
    MENU.with(|state| {
        let mut m = state.borrow_mut();
        if !m.open {
            return ControlFlow::Continue(());
        }
        if !pressed {
            m.held_field = false;
            m.held_slider = None;
            g.dragging = false;
            return ControlFlow::Break(Ok(()));
        }
        if m.help.take().is_some() {
            m.keyboard_action = None;
            return ControlFlow::Break(Ok(()));
        }
        if right {
            return ControlFlow::Break(Ok(()));
        }
        let hit = m
            .hits
            .iter()
            .rev()
            .find(|hit| contains(g.cursor, hit.rect))
            .map(|hit| hit.action.clone());
        if let Some(a) = hit {
            let result = action(&mut m, g, a);
            outcome(&mut m, result);
        } else {
            m.focus = None;
            m.held_field = false;
            m.held_slider = None;
            m.keyboard_action = None;
        }
        ControlFlow::Break(Ok(()))
    })
}
pub(super) fn drag(g: &mut Game<'_>) -> UiResult {
    MENU.with(|state| {
        let mut m = state.borrow_mut();
        if !m.open {
            return ControlFlow::Continue(());
        }
        if let Some((key, track)) = m.held_slider {
            if let Some(range) = slider_range(key) {
                let result = set_slider(
                    &mut m,
                    key,
                    range.value(ui::slider_value(track[0], track[1], g.cursor.0)),
                );
                outcome(&mut m, result);
            }
        } else if m.held_field {
            if let Some(i) = m.focus {
                m.fields[i].edit.drag_to(&g.gui.atlas, g.cursor.0);
            }
        }
        ControlFlow::Break(Ok(()))
    })
}
pub(super) fn key(
    g: &mut Game<'_>,
    key: winit::keyboard::KeyCode,
    typed: Option<&str>,
) -> UiResult {
    use winit::keyboard::KeyCode as K;
    MENU.with(|state| {
        let mut m = state.borrow_mut();
        if !m.open {
            return ControlFlow::Continue(());
        }
        if m.help.is_some() {
            if matches!(key, K::Escape | K::Enter | K::Space) {
                m.help = None;
            }
            return ControlFlow::Break(Ok(()));
        }
        if key == K::Escape {
            let result = close(&mut m, g);
            outcome(&mut m, result);
            return ControlFlow::Break(Ok(()));
        }
        if key == K::Tab {
            let actions: Vec<Action> = m
                .hits
                .iter()
                .filter(|h| h.rect[3] > 0.0)
                .map(|h| h.action.clone())
                .collect();
            if !actions.is_empty() {
                let current = m
                    .keyboard_action
                    .as_ref()
                    .and_then(|a| actions.iter().position(|b| a == b));
                let next = if g.has_shift_down() {
                    current.map_or(actions.len() - 1, |i| {
                        (i + actions.len() - 1) % actions.len()
                    })
                } else {
                    current.map_or(0, |i| (i + 1) % actions.len())
                };
                let action = actions[next].clone();
                m.focus = if let Action::Field(i) = action {
                    Some(i)
                } else {
                    None
                };
                m.keyboard_action = Some(action);
                m.held_field = false;
                m.held_slider = None;
            }
            return ControlFlow::Break(Ok(()));
        }
        let slider_key = match &m.keyboard_action {
            Some(Action::Slider(key, _)) | Some(Action::Step(key, _)) => Some(*key),
            _ => None,
        };
        if let Some(slider_key) = slider_key {
            if matches!(key, K::ArrowLeft | K::ArrowRight | K::Home | K::End) {
                if let Some(range) = slider_range(slider_key) {
                    let value = match key {
                        K::Home => range.min,
                        K::End => range.max,
                        _ => range.nudge(
                            slider_current(&m, slider_key),
                            (if key == K::ArrowLeft { -1.0 } else { 1.0 })
                                * if g.has_shift_down() { 10.0 } else { 1.0 },
                        ),
                    };
                    let result = set_slider(&mut m, slider_key, value);
                    outcome(&mut m, result);
                }
                return ControlFlow::Break(Ok(()));
            }
        }
        if matches!(key, K::Enter | K::NumpadEnter | K::Space) && m.focus.is_none() {
            let selected = m
                .keyboard_action
                .clone()
                .filter(|a| !matches!(a, Action::Slider(_, _) | Action::Field(_)));
            let result = if let Some(selected) = selected {
                action(&mut m, g, selected)
            } else if key != K::Space {
                apply(&mut m, g)
            } else {
                Ok(())
            };
            outcome(&mut m, result);
            return ControlFlow::Break(Ok(()));
        }
        if matches!(key, K::Enter | K::NumpadEnter) {
            let result = apply(&mut m, g);
            outcome(&mut m, result);
            return ControlFlow::Break(Ok(()));
        }
        if let Some(i) = m.focus {
            let before = m.fields[i].edit.value.clone();
            let handled = m.fields[i]
                .edit
                .key(key, g.has_control_down(), g.has_shift_down());
            if !handled && g.types_text() {
                if let Some(value) = typed {
                    m.fields[i].edit.insert_text(value);
                }
            }
            if m.fields[i].edit.value != before {
                if m.fields[i].key == "search" {
                    m.scroll = 0.0;
                } else if m.fields[i].key != "profile_name" {
                    m.dirty = true;
                }
            }
        } else if matches!(key, K::PageDown | K::PageUp | K::ArrowDown | K::ArrowUp) {
            let step = if matches!(key, K::PageDown | K::PageUp) {
                m.viewport[3] * 0.8
            } else {
                22.0
            };
            m.scroll = (m.scroll
                + if matches!(key, K::PageDown | K::ArrowDown) {
                    step
                } else {
                    -step
                })
            .clamp(0.0, (m.content - m.viewport[3]).max(0.0));
        }
        ControlFlow::Break(Ok(()))
    })
}
pub(super) fn shortcut(g: &mut Game<'_>, shortcut: EditShortcut) -> ControlFlow<bool> {
    MENU.with(|state| {
        let mut m = state.borrow_mut();
        if !m.open {
            return ControlFlow::Continue(());
        }
        if m.help.is_none() {
            if let Some(i) = m.focus {
                if m.fields[i].edit.shortcut(shortcut, &mut g.clipboard) {
                    if m.fields[i].key == "search" {
                        m.scroll = 0.0;
                    } else if m.fields[i].key != "profile_name" {
                        m.dirty = true;
                    }
                }
            }
        }
        ControlFlow::Break(true)
    })
}
pub(super) fn scroll(g: &mut Game<'_>, amount: f32) -> UiResult {
    MENU.with(|state| {
        let mut m = state.borrow_mut();
        if !m.open {
            return ControlFlow::Continue(());
        }
        if m.help.is_none() && contains(g.cursor, m.viewport) {
            m.scroll = (m.scroll - amount * 22.0).clamp(0.0, (m.content - m.viewport[3]).max(0.0));
        }
        ControlFlow::Break(Ok(()))
    })
}
