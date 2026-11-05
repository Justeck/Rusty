// Copyright 2026 umbx-dot, MIT; see LICENSE-Camera.rs.
// Modified 2026-11-03 for Rusty by Justeck: shared config, lifecycle and action guards.
use super::{Game, perspective::CameraType};
use horizon_client_render::mesher::{rusty, rusty_freecam as freecam};
use std::cell::Cell;
use std::ops::ControlFlow;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

const MOVES: [KeyCode; 7] = [
    KeyCode::KeyW,
    KeyCode::KeyS,
    KeyCode::KeyA,
    KeyCode::KeyD,
    KeyCode::Space,
    KeyCode::ShiftLeft,
    KeyCode::ControlLeft,
];
const VIEWS: [CameraType; 3] = [
    CameraType::FirstPerson,
    CameraType::ThirdPersonBack,
    CameraType::ThirdPersonFront,
];
thread_local! { static DIMENSION: Cell<Option<&'static str>> = const { Cell::new(None) }; }

fn read_settings() -> freecam::Settings {
    let cfg = rusty::config();
    freecam::Settings {
        noclip: cfg.freecam_noclip,
        speed: cfg.freecam_speed,
    }
}

fn quiet_body(game: &mut Game<'_>) {
    game.keys.retain(|key| !MOVES.contains(key));
    // Reset both the held buttons and toggle state so accessibility toggles
    // cannot re-enable attack/use after a menu closes.
    game.toggles = Default::default();
    game.left = false;
    game.right = false;
    game.attack_clicks = 0;
    game.mining = None;
}

pub(super) fn toggle(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    if freecam::active() {
        exit(game);
        return;
    }
    if !game.native_on(rusty::ID) || !game.native_playing() || !game.focused {
        return;
    }
    // Cancelling an in-progress bow/trident use can itself fire an action.
    // Require ordinary use to finish before entering detached-camera mode.
    if game.using.is_some() {
        game.native_chat("Finish using the held item before enabling Freecam.");
        return;
    }
    let Some(player) = game.player.as_ref() else {
        return;
    };
    let eye = [
        player.position.x,
        player.position.y + f64::from(game.eye),
        player.position.z,
    ];
    let view = VIEWS
        .iter()
        .position(|view| *view == game.camera_type)
        .unwrap_or(0) as u8;
    freecam::set_settings(read_settings());
    freecam::start(eye, game.yaw, game.pitch, view);
    DIMENSION.set(Some(game.dim.id()));
    game.stop_remote_mining();
    quiet_body(game);
    game.camera_type = CameraType::ThirdPersonBack;
}

pub(super) fn exit(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    let Some(view) = freecam::stop() else {
        return;
    };
    DIMENSION.set(None);
    game.camera_type = VIEWS[usize::from(view).min(VIEWS.len() - 1)];
    quiet_body(game);
    // Re-enter ordinary movement from keys that are physically down now,
    // rather than resurrecting a stale pre-freecam movement snapshot.
    if game.native_playing() && game.focused {
        let held: Vec<_> = game
            .physical_keys
            .iter()
            .flat_map(|key| game.settings.keys.translate_all(*key))
            .filter_map(Result::ok)
            .filter(|key| MOVES.contains(key))
            .collect();
        game.keys.extend(held);
    }
}

/// Hook on clear_entities and both local/remote dimension transitions.
pub(super) fn reset(game: &mut Game<'_>) {
    exit(game);
}

/// Per-frame lifecycle guard; keep this in addition to the 20 Hz tick hook.
pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_window() || !freecam::active() {
        return;
    }
    if game.player.is_none()
        || !game.native_on(rusty::ID)
        || DIMENSION.get() != Some(game.dim.id())
        || game.title.is_some()
    {
        exit(game);
        return;
    }
    freecam::suspend(!game.focused || !game.native_playing());
    quiet_body(game);
    // Third-person body rendering suppresses the first-person arm/item path.
    game.camera_type = CameraType::ThirdPersonBack;
}

/// Run once per native_tick on the window Game only. This never writes the
/// player's position, rotation or velocity: normal body gravity still runs.
pub(super) fn tick(game: &mut Game<'_>) {
    frame(game);
    if !game.native_window() || !freecam::active() {
        return;
    }
    let settings = read_settings();
    freecam::set_settings(settings);
    let game = &*game;
    freecam::tick(|[x, y, z]| {
        !settings.noclip
            && game
                .res
                .facts()
                .get(game.world.block(x, y, z))
                .full_collision
    });
}

fn change_speed(game: &mut Game<'_>, steps: f32) {
    freecam::set_settings(read_settings());
    let Some(settings) = freecam::wheel(steps) else {
        return;
    };
    let mut cfg = (*rusty::config()).clone();
    cfg.freecam_speed = settings.speed;
    if let Err(error) = super::rusty_control::apply_config(game, cfg) {
        // Restore the runtime setting as well when persistence rejects it.
        freecam::set_settings(read_settings());
        game.native_chat(&format!("Could not save Freecam speed: {error}"));
    }
}

pub(super) fn wheel(game: &mut Game<'_>, steps: f32, claimed: &mut bool) {
    if !game.native_window() || !freecam::active() || !game.native_playing() || !game.focused {
        return;
    }
    change_speed(game, steps);
    *claimed = true;
}

fn body_action(key: KeyCode) -> bool {
    MOVES.contains(&key)
        || matches!(
            key,
            KeyCode::KeyQ
                | KeyCode::KeyF
                | KeyCode::Digit1
                | KeyCode::Digit2
                | KeyCode::Digit3
                | KeyCode::Digit4
                | KeyCode::Digit5
                | KeyCode::Digit6
                | KeyCode::Digit7
                | KeyCode::Digit8
                | KeyCode::Digit9
        )
}

fn native_shortcut(game: &Game<'_>, code: KeyCode) -> bool {
    let Some(name) = horizon_client::keybinds::key_name(code) else {
        return false;
    };
    native_shortcut_name(game, name)
}

fn native_shortcut_name(game: &Game<'_>, name: &str) -> bool {
    [
        ("xray", "key.keyboard.x"),
        ("menu", "key.keyboard.right.shift"),
        ("freecam", "key.keyboard.u"),
        ("fullbright", "key.keyboard.b"),
        ("finder", "key.keyboard.v"),
        ("zoom", "key.keyboard.c"),
    ]
    .iter()
    .any(|(binding, default)| game.native_mod_key(rusty::ID, binding, default) == name)
}

/// The native key hook is also used by Horizon's authenticated control
/// interface. Raw camera movement is handled earlier by the window hook;
/// routing these names keeps both input paths consistent.
pub(super) fn key(game: &mut Game<'_>, name: &str, pressed: bool) {
    if !game.native_window()
        || !freecam::active()
        || !game.native_playing()
        || !game.focused
        || native_shortcut_name(game, name)
    {
        return;
    }
    if let Some(input) = MOVES
        .iter()
        .position(|code| horizon_client::keybinds::key_name(*code) == Some(name))
    {
        freecam::suspend(false);
        freecam::hold(input, pressed);
    }
}

/// Window hook preserves chat, menus and global shortcuts. Camera movement
/// uses WASD independently of the body's remapped movement bindings.
pub(super) fn window(game: &mut Game<'_>, event: &WindowEvent) -> ControlFlow<()> {
    if !game.native_window() || !freecam::active() {
        return ControlFlow::Continue(());
    }
    if matches!(event, WindowEvent::Focused(false)) {
        freecam::suspend(true);
        quiet_body(game);
        return ControlFlow::Continue(());
    }
    if !game.native_playing() || !game.focused {
        freecam::suspend(true);
        return ControlFlow::Continue(());
    }
    freecam::suspend(false);
    match event {
        WindowEvent::MouseInput { button, .. } => {
            if native_shortcut_name(game, &horizon_client::keybinds::mouse_name(*button)) {
                return ControlFlow::Continue(());
            }
            quiet_body(game);
            ControlFlow::Break(())
        }
        WindowEvent::KeyboardInput { event, .. } => {
            let PhysicalKey::Code(code) = event.physical_key else {
                return ControlFlow::Continue(());
            };
            let down = event.state == ElementState::Pressed;
            // Native dispatch must see both press and release, even when the
            // user binds a module shortcut to a movement/action key.
            if native_shortcut(game, code) {
                return ControlFlow::Continue(());
            }
            if let Some(input) = MOVES.iter().position(|key| *key == code) {
                freecam::hold(input, down);
                if down {
                    game.physical_keys.insert(code);
                } else {
                    game.physical_keys.remove(&code);
                }
                return ControlFlow::Break(());
            }
            if matches!(code, KeyCode::PageUp | KeyCode::PageDown) {
                if down && !event.repeat {
                    change_speed(game, if code == KeyCode::PageUp { 1.0 } else { -1.0 });
                }
                return ControlFlow::Break(());
            }
            if game
                .settings
                .keys
                .translate_all(code)
                .iter()
                .any(|mapped| match mapped {
                    Ok(key) => body_action(*key),
                    Err(_) => true,
                })
            {
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        }
        _ => ControlFlow::Continue(()),
    }
}

/// Optional mapped_key HEAD guard also covers controller/control-socket keys.
pub(super) fn mapped_key(game: &Game<'_>, key: KeyCode) -> bool {
    game.native_window() && freecam::active() && game.native_playing() && body_action(key)
}

/// play_mouse HEAD guard also covers keyboard-rebound attack and scripted input.
pub(super) fn mouse(game: &Game<'_>) -> bool {
    game.native_window() && freecam::active() && game.native_playing()
}

/// pad_move RETURN guard: suppress body analog motion while detached.
pub(super) fn pad_motion(game: &Game<'_>, value: Option<[f32; 2]>) -> Option<[f32; 2]> {
    if game.native_window() && freecam::active() {
        None
    } else {
        value
    }
}

pub(super) fn turn(xo: f64, yo: f64) -> ControlFlow<()> {
    if freecam::active() {
        freecam::turn(xo, yo);
        ControlFlow::Break(())
    } else {
        ControlFlow::Continue(())
    }
}

pub(super) fn camera(partial: f32) -> ControlFlow<Option<(glam::DVec3, f32, f32)>> {
    freecam::view(partial).map_or(ControlFlow::Continue(()), |([x, y, z], yaw, pitch)| {
        ControlFlow::Break(Some((glam::DVec3::new(x, y, z), yaw, pitch)))
    })
}
