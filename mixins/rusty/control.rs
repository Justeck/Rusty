// Rusty integration, Copyright 2026 Justeck, Apache-2.0.
// Original copyright 1_st_Cat (RealCat1st); see NOTICE.rs for attribution.
use super::Game;
use horizon_client_render::mesher::rusty::{self, Activation, ActivationMode, Config};
use std::cell::{Cell, RefCell};
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::PhysicalKey;
thread_local! {
 static INPUT: RefCell<Activation> = RefCell::new(Activation::default());
 static MINE_REQUEST: Cell<u8> = const { Cell::new(0) };
 static WALK_REQUEST: Cell<bool> = const { Cell::new(false) };
 static LOADED: Cell<bool> = const { Cell::new(false) };
 static ZOOM: Cell<bool> = const { Cell::new(false) };
 static SPRINT: Cell<bool> = const { Cell::new(false) };
 static FRESH: RefCell<super::rusty_ui_controls::FreshPressGate> = RefCell::new(Default::default());
}
pub(super) fn close_menu(game: &mut Game<'_>) {
    game.native_mod_screen_close(rusty::ID);
}
/// Invoked at both native screen transitions, including controller closes.
pub(super) fn consume_menu_inputs(game: &mut Game<'_>) {
    if !game.native_window() || game.native_mod_screen() != Some(rusty::ID) {
        return;
    }
    consume_inputs(game);
}
/// The advanced editor owns an Options surface, so its transitions call this
/// directly without inspecting the editor's already-borrowed UI state.
pub(super) fn consume_editor_inputs(game: &mut Game<'_>) {
    if game.native_window() && game.native_on(rusty::ID) {
        consume_inputs(game);
    }
}
fn consume_inputs(game: &mut Game<'_>) {
    FRESH.with_borrow_mut(|gate| {
        gate.consume(
            game.physical_keys
                .iter()
                .filter_map(|key| horizon_client::keybinds::key_name(*key)),
        )
    });
    game.physical_keys.clear();
    game.physical_buttons.clear();
    game.keys.clear();
    game.toggles = Default::default();
    game.left = false;
    game.right = false;
    game.attack_clicks = 0;
    game.mining = None;
    ZOOM.set(false);
    INPUT.with_borrow_mut(|input| {
        input.down = false;
        if rusty::config().activation == ActivationMode::Hold
            && input.cancel()
            && rusty::set_active(false)
        {
            refresh(game);
        }
    });
    horizon_client_render::mesher::rusty_freecam::suspend(true);
}
pub(super) fn observe_key(
    game: &Game<'_>,
    key: winit::keyboard::KeyCode,
    pressed: bool,
    repeat: bool,
) {
    if pressed && !repeat {super::rusty_mining_native::manual_look();}
    if game.native_window()
        && let Some(name) = horizon_client::keybinds::key_name(key)
    {
        FRESH.with_borrow_mut(|gate| {
            gate.key(name, pressed, repeat);
        });
    }
}
pub(super) fn block_mapped(
    game: &Game<'_>,
    raw: winit::keyboard::KeyCode,
    pressed: bool,
    repeat: bool,
) -> bool {
    game.native_window()
        && game.native_on(rusty::ID)
        && game.native_playing()
        && horizon_client::keybinds::key_name(raw)
            .is_some_and(|name| FRESH.with_borrow_mut(|gate| gate.key(name, pressed, repeat)))
}
pub(super) fn before_restore(game: &mut Game<'_>) {
    if !game.native_window() || !game.native_on(rusty::ID) {
        return;
    }
    FRESH.with_borrow(|gate| {
        game.physical_keys.retain(|key| {
            horizon_client::keybinds::key_name(*key).is_none_or(|name| !gate.blocked(name))
        })
    });
}
pub(super) fn blocked_repeat(game: &Game<'_>, raw: winit::keyboard::KeyCode) -> bool {
    game.native_window()
        && game.native_on(rusty::ID)
        && game.native_playing()
        && horizon_client::keybinds::key_name(raw)
            .is_some_and(|name| FRESH.with_borrow(|gate| gate.blocked(name)))
}
/// Release bookkeeping for OS, controller and synthesized window events.
/// Presses remain behind the existing context and fresh-input gates.
pub(super) fn observe_release(game: &Game<'_>, event: &WindowEvent) {
    if !game.native_window() || !game.native_on(rusty::ID) {
        return;
    }
    // Native mouse-key callbacks run only in Playing. Observe releases before
    // a screen consumes them; release bookkeeping never grants an action.
    match event {
        WindowEvent::MouseInput {
            state: ElementState::Released,
            button,
            ..
        } => {
            let name = horizon_client::keybinds::mouse_name(*button);
            super::rusty_inventory::input(game, &name, false);
        }
        WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Released => {
            if let PhysicalKey::Code(code) = event.physical_key
                && let Some(name) = horizon_client::keybinds::key_name(code)
            {
                FRESH.with_borrow_mut(|gate| {
                    gate.key(name, false, false);
                });
                super::rusty_inventory::input(game, name, false);
            }
        }
        _ => {}
    }
}
pub(super) fn window_blocked(game: &Game<'_>, event: &WindowEvent) -> bool {
    if !game.native_window() || !game.native_on(rusty::ID) {
        return false;
    }
    observe_release(game, event);
    if game.native_mod_screen() == Some(rusty::ID) {
        return false;
    }
    if let WindowEvent::KeyboardInput {
        event,
        is_synthetic,
        ..
    } = event
        && let PhysicalKey::Code(code) = event.physical_key
        && let Some(name) = horizon_client::keybinds::key_name(code)
    {
        return FRESH.with_borrow_mut(|gate| {
            gate.key(
                name,
                event.state == ElementState::Pressed,
                event.repeat || *is_synthetic,
            )
        });
    }
    false
}
pub(super) fn pad_motion(
    game: &Game<'_>,
    value: Option<[f32; 2]>,
    focused: bool,
) -> Option<[f32; 2]> {
    if game.native_window() && game.native_on(rusty::ID) && game.native_playing() {
        let raw = game.pad.state.left;
        let centered = game.settings.controller.enabled
            && focused
            && raw.into_iter().all(f32::is_finite)
            && horizon_client::gamepad::radial(raw, game.settings.controller.move_deadzone)
                == [0., 0.];
        FRESH.with_borrow_mut(|gate| gate.movement(value, centered))
    } else {
        value
    }
}
pub(super) fn pad_look_blocked(game: &Game<'_>, focused: bool) -> bool {
    if !game.native_window() || !game.native_on(rusty::ID) || !game.native_playing() {
        return false;
    }
    let centered = game.settings.controller.enabled
        && focused
        && game.pad.state.right.into_iter().all(f32::is_finite)
        && horizon_client::gamepad::radial(
            game.pad.state.right,
            game.settings.controller.look_deadzone,
        ) == [0., 0.];
    !FRESH.with_borrow_mut(|gate| gate.look(centered))
}
fn refresh(game: &mut Game<'_>) {
    horizon_client_render::streaming::rusty_stream::invalidate(
        &mut game.streaming,
        &mut game.world,
    );
    game.pending_uploads.clear();
    if let Some(renderer) = &mut game.renderer {
        renderer.clear_sections(game.world.layout());
    }
    game.clear_lod();
    game.all_sections_changed();
    super::rusty_visuals::reset();
}
fn brightness(game: &mut Game<'_>) {
    let enabled = game.native_on(rusty::ID);
    let cfg = rusty::config();
    if let Some(r) = game.renderer.as_mut() {
        r.fullbright = game.settings.extras.fullbright
            || (enabled && ((rusty::active() && !cfg.normal_terrain()) || cfg.fullbright));
    }
}
/// GUI activation follows the same state and mesh invalidation as the hotkey.
pub(super) fn toggle_xray(game: &mut Game<'_>) {
    if !game.native_on(rusty::ID) || game.native_player().is_none() {
        return;
    }
    if rusty::config().activation == ActivationMode::Hold {
        game.native_chat("Xray uses Hold mode: close the menu and hold its bound key.");
        return;
    }
    INPUT.with_borrow_mut(|activation| {
        activation.active = !rusty::active();
        activation.down = false;
        if rusty::set_active(activation.active) {
            refresh(game);
        }
    });
    brightness(game);
}
pub(super) fn toggle_freecam(game: &mut Game<'_>) {
    close_menu(game);
    super::rusty_camera::toggle(game);
}
pub(super) fn default_module(game: &mut Game<'_>, id: &str) {
    match id {
        "xray" => {
            INPUT.with_borrow_mut(|a| a.cancel());
            let was = rusty::active();
            rusty::set_active(false);
            if was {
                refresh(game);
            }
        }
        "freecam" => super::rusty_camera::exit(game),
        "zoom" => ZOOM.set(false),
        "waypoint_walk" => super::rusty_waypoint_walk::reset(game),
        "auto_mine" => super::rusty_mining_native::reset(game),
        _ => {}
    }
}
pub(super) fn default_runtime(game: &mut Game<'_>) {
    for id in ["xray", "freecam", "zoom", "waypoint_walk", "auto_mine"] {
        default_module(game, id);
    }
    super::rusty_assist::reset(game);
    super::rusty_inventory::reset();
    super::rusty_automations::reset();
    super::rusty_builder::reset();
}
pub(super) fn apply_config(game: &mut Game<'_>, cfg: Config) -> Result<(), String> {
    let cfg = cfg.validated()?;
    let previous = rusty::config();
    let selection_changed = previous.rules != cfg.rules;
    let terrain_mode_changed = previous.normal_terrain() != cfg.normal_terrain();
    let mode_changed = previous.activation != cfg.activation;
    let text = cfg.to_json()?;
    let dir = game.native_data_dir(rusty::ID);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create settings folder: {e}"))?;
    let temporary = dir.join("config.tmp");
    std::fs::write(&temporary, text)
        .and_then(|()| std::fs::rename(&temporary, dir.join("config")))
        .map_err(|e| format!("Could not save settings: {e}"))?;
    rusty::set_config(cfg)?;
    if mode_changed {
        INPUT.with_borrow_mut(|a| {
            a.cancel();
        });
        rusty::set_active(false);
    }
    if ((selection_changed || terrain_mode_changed) && rusty::active()) || mode_changed {
        refresh(game)
    }
    brightness(game);
    Ok(())
}
fn load(game: &mut Game<'_>) {
    if LOADED.replace(true) {
        return;
    }
    let path = game.native_data_dir(rusty::ID).join("config");
    match std::fs::File::open(path) {
        Ok(file) => match Config::read_json(file).and_then(rusty::set_config) {
            Ok(()) => (),
            Err(e) => game.native_chat(&format!(
                "Rusty settings could not load; using defaults: {e}"
            )),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => game.native_chat(&format!("Rusty settings could not be read: {error}")),
    }
}
pub(super) fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !game.native_window() {
        return;
    }
    if !pressed {
        FRESH.with_borrow_mut(|gate| {
            gate.key(key, false, false);
        });
    } else if game.native_playing() && FRESH.with_borrow(|gate| gate.blocked(key)) {
        return;
    }
    load(game);
    super::rusty_inventory::input(game, key, pressed);
    super::rusty_camera::key(game, key, pressed);
    let playing = game.native_on(rusty::ID) && game.native_playing() && game.focused;
    if key == game.native_mod_key(rusty::ID, "xray", rusty::DEFAULT_KEY) {
        let mode = rusty::config().activation;
        INPUT.with_borrow_mut(|a| {
            a.event(mode, pressed, playing);
            if rusty::set_active(a.active) {
                refresh(game);
            }
        });
        brightness(game);
    }
    if key == game.native_mod_key(rusty::ID, "zoom", "key.keyboard.c") {
        ZOOM.set(pressed && playing);
    }
    if !pressed || !playing {
        return;
    }
    if key == game.native_mod_key(rusty::ID, "menu", "key.keyboard.right.shift") {
        super::rusty_clickgui::open(game);
    }
    if key == game.native_mod_key(rusty::ID, "freecam", "key.keyboard.u") {
        super::rusty_camera::toggle(game);
    }
    let mine_binding = game.native_mod_key(rusty::ID, "auto_mine", "key.keyboard.unknown");
    if mine_binding != "key.keyboard.unknown" && key == mine_binding {
        if let Err(error) = super::rusty_mining_native::toggle(game) { game.native_chat(&error); }
    }
    let walk_binding = game.native_mod_key(rusty::ID, "waypoint_walk", "key.keyboard.unknown");
    if walk_binding != "key.keyboard.unknown" && key == walk_binding {
        if let Err(error) = super::rusty_waypoint_walk::toggle(game) {
            game.native_chat(&error);
        }
    }
    let mut cfg = (*rusty::config()).clone();
    let mut changed = false;
    if key == game.native_mod_key(rusty::ID, "fullbright", "key.keyboard.b") {
        cfg.fullbright = !cfg.fullbright;
        changed = true;
    }
    if key == game.native_mod_key(rusty::ID, "finder", "key.keyboard.v") {
        cfg.finder_enabled = !cfg.finder_enabled;
        changed = true;
    }
    for module in horizon_client_render::mesher::rusty_catalog::MODULES {
        if matches!(
            module.id,
            "xray" | "menu" | "freecam" | "fullbright" | "finder" | "zoom"
        ) {
            continue;
        }
        if let Some(id) = module.keybind_id {
            let binding = game.native_mod_key(
                rusty::ID,
                id,
                module.default_key.unwrap_or("key.keyboard.unknown"),
            );
            if binding != "key.keyboard.unknown" && key == binding {
                if let Some(value) = super::rusty_clickgui::config_bool(&mut cfg, module.id) {
                    *value = !*value;
                    changed = true;
                }
            }
        }
    }
    if changed && let Err(e) = apply_config(game, cfg) {
        game.native_chat(&e);
    }
}
pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    load(game);
    rusty::set_visual_enabled(game.native_on(rusty::ID) && game.native_player().is_some());
    if !game.native_on(rusty::ID) || game.native_player().is_none() {
        reset(game);
    }
    if !game.native_playing() || !game.focused || super::rusty_ui::is_open() {
        ZOOM.set(false);
        INPUT.with_borrow_mut(|a| {
            a.down = false;
            if rusty::config().activation == ActivationMode::Hold
                && a.cancel()
                && rusty::set_active(false)
            {
                refresh(game);
            }
        });
    }
    match MINE_REQUEST.replace(0) {1 if !super::rusty_mining_native::active()=>{if let Err(e)=super::rusty_mining_native::toggle(game){game.native_chat(&e);}},2=>super::rusty_mining_native::reset(game),_=>{}}
    if WALK_REQUEST.replace(false) {
        if let Err(error) = super::rusty_waypoint_walk::toggle(game) {
            game.native_chat(&error);
        }
    }
    super::rusty_waypoint_walk::frame(game);
    super::rusty_mining_native::frame(game);
    super::rusty_clickgui::frame(game);
    super::rusty_ui::frame(game);
    super::rusty_navigation_native::frame(game);
    super::rusty_builder::frame(game);
    super::rusty_assist::frame(game);
    super::rusty_automations::frame(game);
    super::rusty_inventory::frame(game);
    super::rusty_awareness_native::frame(game);
    super::rusty_camera::frame(game);
    super::rusty_visuals::frame(game);
    brightness(game);
}
pub(super) fn tick(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    super::rusty_camera::tick(game);
}
// API 359 calls native_tick after remote movement. Request the ordinary sprint
// key at the start of Game::tick, after latch healing and before physics.
pub(super) fn sprint(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    use winit::keyboard::KeyCode;
    let held = game.physical_keys.iter().any(|key| {
        game.settings
            .keys
            .translate_all(*key)
            .iter()
            .any(|mapped| matches!(mapped, Ok(KeyCode::ControlLeft)))
    });
    let want = game.native_on(rusty::ID)
        && !super::rusty_mining_native::active()
        && rusty::config().auto_sprint
        && game.native_playing()
        && game.focused
        && !horizon_client_render::mesher::rusty_freecam::active()
        && game.keys.contains(&KeyCode::KeyW);
    if SPRINT.replace(want) && !want && !held {
        game.keys.remove(&KeyCode::ControlLeft);
    }
    if want {
        game.keys.insert(KeyCode::ControlLeft);
    }
}
pub(super) fn reset(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    INPUT.with_borrow_mut(|a| {
        a.cancel();
    });
    ZOOM.set(false);
    super::rusty_navigation_native::reset();
    super::rusty_waypoint_walk::reset(game);
    super::rusty_mining_native::reset(game);
    WALK_REQUEST.set(false);
    MINE_REQUEST.set(0);
    super::rusty_builder::reset();
    super::rusty_assist::reset(game);
    super::rusty_automations::reset();
    super::rusty_inventory::reset();
    super::rusty_awareness_native::reset();
    super::rusty_camera::reset(game);
    if rusty::set_active(false) {
        refresh(game)
    } else {
        super::rusty_visuals::reset()
    }
    brightness(game);
}
pub(super) fn window(game: &mut Game<'_>, event: &WindowEvent) {
    if matches!(event,WindowEvent::MouseInput{state:ElementState::Pressed,..}|WindowEvent::MouseWheel{..}){super::rusty_mining_native::manual_look();}
    if !game.native_window() {
        return;
    }
    if matches!(event, WindowEvent::Focused(false)) {
        super::rusty_mining_native::reset(game);
        INPUT.with_borrow_mut(|a| {
            a.down = false;
            if rusty::config().activation == ActivationMode::Hold {
                a.cancel();
                if rusty::set_active(false) {
                    refresh(game)
                }
            }
        });
        ZOOM.set(false);
    }
    let released = match event {
        WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Released => {
            match event.physical_key {
                PhysicalKey::Code(code) => {
                    horizon_client::keybinds::key_name(code).map(str::to_owned)
                }
                _ => None,
            }
        }
        WindowEvent::MouseInput {
            state: ElementState::Released,
            button,
            ..
        } => Some(horizon_client::keybinds::mouse_name(*button)),
        _ => None,
    };
    if let Some(name) = released {
        key(game, &name, false);
    }
}
pub(super) fn zoom_active() -> bool {
    ZOOM.get()
}
pub(super) fn zoom(game: &Game<'_>, out: &mut Option<super::native_hooks::NativeZoom>) {
    if out.is_none()
        && game.native_window()
        && game.native_on(rusty::ID)
        && game.native_playing()
        && ZOOM.get()
    {
        let factor = rusty::config().zoom_factor;
        *out = Some(super::native_hooks::NativeZoom {
            factor,
            smooth: true,
            turn: 1.0 / factor,
        });
    }
}
pub(super) fn enabled(game: &Game<'_>) -> bool {
    game.native_window() && game.native_on(rusty::ID)
}
pub(super) fn settings_applied(game: &mut Game<'_>) {
    brightness(game);
}
pub(super) fn command(game: &mut Game<'_>, command: &str, ran: &mut bool) {
    if !enabled(game) {
        return;
    }
    let Some(args) = command
        .strip_prefix("rusty")
        .filter(|s| s.is_empty() || s.starts_with(' '))
    else {
        return;
    };
    *ran = true;
    match args.trim() {
        "" | "menu" => super::rusty_clickgui::open(game),
        "settings" => super::rusty_ui::open(game, true),
        "mine start" => MINE_REQUEST.set(1),
        "mine stop" => MINE_REQUEST.set(2),
        "mine status" => game.native_chat(&super::rusty_mining_native::status().to_string()),
        "scan" => {
            let mut cfg = (*rusty::config()).clone();
            cfg.finder_enabled = true;
            match apply_config(game, cfg) {
                Ok(()) => super::rusty_visuals::request_scan(),
                Err(e) => game.native_chat(&e),
            }
        }
        "status" => {
            let s = super::rusty_visuals::stats();
            game.native_chat(&format!(
                "Rusty: xray={} targets={} pending={} partial={}",
                rusty::active(),
                s.total,
                s.pending,
                s.truncated
            ));
        }
        args if args.starts_with("waypoint ") => {
            let result = waypoint_command(game, args.trim_start_matches("waypoint ").trim());
            if let Err(error) = result {
                game.native_chat(&error);
            }
        }
        _ => game.native_chat("Rusty: /rusty, /rusty settings, /rusty scan, /rusty status, /rusty mine start|stop|status, /rusty waypoint add NAME|list|select ID|delete ID|copy ID"),
    }
}

fn waypoint_command(game: &mut Game<'_>, args: &str) -> Result<(), String> {
    use super::rusty_navigation_native as navigation;
    let (action, value) = args.split_once(' ').unwrap_or((args, ""));
    let parse_id = || {
        value
            .trim()
            .parse::<u64>()
            .map_err(|_| "Use a waypoint ID from /rusty waypoint list".to_owned())
    };
    match action {
        "add" => {
            let id = navigation::add_current(game, value.trim())?;
            game.native_chat(&format!(
                "Waypoint {id} saved and selected in this world and dimension."
            ));
        }
        "walk" => {
            WALK_REQUEST.set(true);
        }
        "list" => {
            let points = navigation::waypoints(game)?;
            if points.is_empty() {
                game.native_chat("No waypoints in this world and dimension.");
            }
            for point in points {
                game.native_chat(&format!(
                    "{}: {} ({:.1}, {:.1}, {:.1})",
                    point.id, point.name, point.position[0], point.position[1], point.position[2]
                ));
            }
        }
        "select" if value.trim() == "none" => navigation::select(game, None)?,
        "select" => navigation::select(game, Some(parse_id()?))?,
        "delete" => navigation::remove(game, parse_id()?)?,
        "copy" => {
            navigation::copy(game, parse_id()?)?;
            game.native_chat("Waypoint coordinates copied.");
        }
        _ => return Err("Waypoints: add NAME, list, select ID|none, delete ID, copy ID".into()),
    }
    Ok(())
}

/// Preserve Horizon's optional authenticated control interface for custom
/// native text fields. Ordinary game input still follows the window hooks.
pub(super) fn control(
    game: &mut Game<'_>,
    line: &str,
) -> std::ops::ControlFlow<horizon_client::Result<Option<serde_json::Value>>> {
    use std::ops::ControlFlow;
    if !enabled(game) {
        return ControlFlow::Continue(());
    }
    if matches!(line.trim(), "release" | "stop") {
        ZOOM.set(false);
        INPUT.with_borrow_mut(|input| {
            input.down = false;
            if rusty::config().activation == ActivationMode::Hold
                && input.cancel()
                && rusty::set_active(false)
            {
                refresh(game);
            }
        });
        horizon_client_render::mesher::rusty_freecam::suspend(true);
        if game.native_playing() && game.focused {
            horizon_client_render::mesher::rusty_freecam::suspend(false);
        }
        // The stock control handler still clears its body keys/actions.
    }
    if line.trim() == "rusty-editor-ui" {
        return ControlFlow::Break(Ok(Some(super::rusty_ui::snapshot())));
    }
    if line.trim() == "rusty-status" {
        let scan = super::rusty_visuals::stats();
        return ControlFlow::Break(Ok(Some(serde_json::json!({
            "xray":rusty::active(),
            "enabled_mods":rusty::config().enabled_mods,
            "active_modules":super::rusty_visuals::enabled_module_names(&rusty::config()),
            "freecam":horizon_client_render::mesher::rusty_freecam::active(),
            "camera":horizon_client_render::mesher::rusty_freecam::view(game.clock.camera_fraction() as f32),
            "fullbright":game.renderer.as_ref().is_some_and(|r|r.fullbright),
            "targets":scan.total,"pending":scan.pending,"partial":scan.truncated,
            "scan_cache":{"aggregations":scan.aggregations,"reused":scan.reused},
            "nearest":scan.nearest,
            "awareness":super::rusty_awareness_native::snapshot(),
            "assistance":super::rusty_assist::status(game),
            "automations":super::rusty_automations::status(),
            "build":super::rusty_builder::snapshot(),
            "inventory_assist":super::rusty_inventory::status(),
            "minimap":game.native_minimap_now().map(|m|serde_json::json!({"size":m.size,"scale":m.blocks_per_pixel,"rotate":m.rotate,"round":m.round})),
            "walking":super::rusty_waypoint_walk::status(),
            "mining":super::rusty_mining_native::status(),
        }))));
    }
    if line.trim() == "rusty-build-status" {
        return ControlFlow::Break(Ok(Some(super::rusty_builder::snapshot())));
    }
    if matches!(line.trim(), "rusty-build-anchor" | "rusty-build-place") {
        let result = if line.trim() == "rusty-build-anchor" {
            super::rusty_builder::anchor(game)
        } else {
            super::rusty_builder::place_next(game)
        };
        return ControlFlow::Break(Ok(Some(match result {
            Ok(()) => super::rusty_builder::snapshot(),
            Err(error) => serde_json::json!({"error":error}),
        })));
    }
    if line.trim() == "rusty-mine-status" {return ControlFlow::Break(Ok(Some(super::rusty_mining_native::status())));}
    if line.trim() == "rusty-mine-toggle" {return ControlFlow::Break(Ok(Some(match super::rusty_mining_native::toggle(game){Ok(())=>super::rusty_mining_native::status(),Err(error)=>serde_json::json!({"error":error})})));}
    if line.trim() == "rusty-walk-status" {
        return ControlFlow::Break(Ok(Some(super::rusty_waypoint_walk::status())));
    }
    if line.trim() == "rusty-walk-toggle" {
        return ControlFlow::Break(Ok(Some(match super::rusty_waypoint_walk::toggle(game) {
            Ok(()) => super::rusty_waypoint_walk::status(),
            Err(error) => serde_json::json!({"error":error}),
        })));
    }
    if line.trim() == "rusty-ui-state" {
        return ControlFlow::Break(Ok(Some(super::rusty_clickgui::snapshot())));
    }
    if game.native_mod_screen() == Some(rusty::ID)
        && super::rusty_clickgui::is_open()
        && let Some(text) = line.trim_start().strip_prefix("type ")
    {
        if text == "\\b" {
            super::rusty_clickgui::key(game, "key.keyboard.backspace", true, false, 0);
        } else {
            for character in text.chars().filter(|c| !c.is_control()).take(4096) {
                super::rusty_clickgui::character(game, character);
            }
        }
        return ControlFlow::Break(Ok(Some(serde_json::json!({"ok":true}))));
    }
    if super::rusty_ui::is_open()
        && let Some(text) = line.trim_start().strip_prefix("type ")
    {
        let result = super::rusty_ui::key(game, winit::keyboard::KeyCode::F24, Some(text));
        return match result {
            ControlFlow::Break(result) => {
                ControlFlow::Break(result.map(|()| Some(serde_json::json!({"ok":true}))))
            }
            ControlFlow::Continue(()) => ControlFlow::Continue(()),
        };
    }
    ControlFlow::Continue(())
}
