// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Transient, explicitly started walking to a selected waypoint; no saved start flag.
use super::agent::rusty_waypoint_walk_agent as host;
use super::{Game, rusty_waypoint_walk_state as guard};
use horizon_client_render::mesher::{rusty, rusty_freecam};
use std::{
    cell::RefCell,
    sync::mpsc::{Receiver, TryRecvError},
};
use winit::{event::MouseButton, keyboard::KeyCode};

struct Run {
    selected: guard::Selection,
    name: String,
    completion: Receiver<String>,
}
#[derive(Default)]
struct State {
    run: Option<Run>,
    result: Option<String>,
    arrived: bool,
}
thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

fn available(game: &Game<'_>) -> bool {
    game.native_window()
        && game.native_on(rusty::ID)
        && game.riding.is_none()
        && !game.spectator()
        && game.player.as_ref().is_some_and(|p| {
            !p.health.dead_or_dying() && !p.sleeping && !p.flying && !p.fall_flying
        })
}
fn manual(game: &Game<'_>) -> bool {
    let controller = &game.settings.controller;
    let pad = controller.enabled
        && (game.pad_move().is_some()
            || horizon_client::gamepad::move_vector(
                game.pad.state.right,
                controller.look_deadzone,
            )
            .is_some()
            || game.pad.state.buttons != 0
            || game
                .pad
                .state
                .triggers
                .iter()
                .any(|value| value.abs() > controller.trigger_threshold)
            || [
                "key.jump",
                "key.sneak",
                "key.sprint",
                "key.attack",
                "key.use",
            ]
            .into_iter()
            .any(|action| game.pad.holds_action(controller, action)));
    pad || game
        .physical_buttons
        .iter()
        .any(|button| matches!(button, MouseButton::Left | MouseButton::Right))
        || game.physical_keys.iter().any(|raw| {
            game.settings.keys.translate_all(*raw).iter().any(|mapped| {
                matches!(
                    mapped,
                    Ok(KeyCode::KeyW
                        | KeyCode::KeyS
                        | KeyCode::KeyA
                        | KeyCode::KeyD
                        | KeyCode::Space
                        | KeyCode::ShiftLeft
                        | KeyCode::ControlLeft)
                        | Err(MouseButton::Left | MouseButton::Right)
                )
            })
        })
}
fn selection(game: &Game<'_>) -> Result<Option<(guard::Selection, String)>, String> {
    Ok(
        super::rusty_navigation_native::selected(game)?.map(|(scope, waypoint)| {
            (
                guard::Selection {
                    identity: scope.identity,
                    dimension: scope.dimension,
                    waypoint: waypoint.id,
                    position: waypoint.position,
                },
                waypoint.name,
            )
        }),
    )
}
fn stopped(reason: guard::StopReason) -> &'static str {
    match reason {
        guard::StopReason::Unavailable => "Walking stopped: the player or world is unavailable",
        guard::StopReason::Focus => "Walking stopped: the window lost focus",
        guard::StopReason::Menu => "Walking stopped: a menu opened",
        guard::StopReason::Freecam => "Walking stopped: Freecam is active",
        guard::StopReason::Manual => "Walking stopped: manual input took over",
        guard::StopReason::Selection => "Walking stopped: the world or selected waypoint changed",
        guard::StopReason::Replaced => "Walking stopped: another action took over",
    }
}

pub(super) fn active() -> bool {
    STATE.with_borrow(|state| state.run.is_some())
}

pub(super) fn toggle(game: &mut Game<'_>) -> Result<(), String> {
    if active() {
        reset(game);
        return Ok(());
    }
    if !available(game) || !game.native_playing() || !game.focused || rusty_freecam::active() {
        return Err("Open the world, close menus and leave Freecam before walking".into());
    }
    if manual(game) || game.using.is_some() || game.left || game.right {
        return Err("Release movement and use/attack inputs before walking".into());
    }
    let (selected, name) = selection(game)?.ok_or("Select a waypoint in this world first")?;
    let completion = host::start(game, selected.position)?;
    STATE.with_borrow_mut(|state| {
        state.run = Some(Run {
            selected,
            name,
            completion,
        });
        state.result = None;
        state.arrived = false;
    });
    Ok(())
}

/// Also runs before the engine advances tasks, so physical input wins in the
/// same iteration and the generic task cleanup cannot swallow the user's press.
pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_window() || !active() {
        return;
    }
    STATE.with_borrow_mut(|state| {
        let Some(run) = state.run.as_ref() else {
            return;
        };
        match run.completion.try_recv() {
            Ok(reply) => {
                let value = serde_json::from_str::<serde_json::Value>(&reply).ok();
                state.arrived = value.as_ref().is_some_and(|v| v["arrived"] == true);
                state.result = Some(if state.arrived {
                    "Arrived at waypoint".into()
                } else {
                    value
                        .as_ref()
                        .and_then(|v| v["error"].as_str())
                        .unwrap_or("Walking finished without reaching the waypoint")
                        .chars()
                        .take(240)
                        .collect()
                });
                state.run = None;
                return;
            }
            Err(TryRecvError::Disconnected) => {
                state.run = None;
                state.result = Some("Walking stopped: its movement task ended".into());
                return;
            }
            Err(TryRecvError::Empty) => {}
        }
        let current = if available(game) {
            selection(game).ok().flatten()
        } else {
            None
        };
        let context = guard::Context {
            available: available(game),
            focused: game.focused,
            playing: game.native_playing(),
            freecam: rusty_freecam::active(),
            manual: manual(game),
            selection: current.as_ref().map(|(selected, _)| selected),
            task_name: host::current(game),
        };
        match guard::decision(Some(&run.selected), &context) {
            guard::Decision::Cancel(reason) => {
                host::cancel(game);
                state.run = None;
                state.result = Some(stopped(reason).into());
            }
            guard::Decision::Relinquish => {
                state.run = None;
                state.result = Some("Walking stopped: another action took over".into());
            }
            _ => {}
        }
    });
}

pub(super) fn reset(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    let owned = STATE.with_borrow_mut(|state| {
        let owned = state.run.take().is_some();
        if owned {
            state.result = Some("Waypoint walking stopped".into());
        }
        state.arrived = false;
        owned
    });
    if owned {
        host::cancel(game);
    }
}

pub(super) fn status() -> serde_json::Value {
    STATE.with_borrow(|state| {
        serde_json::json!({
            "active": state.run.is_some(), "arrived": state.arrived,
            "waypoint": state.run.as_ref().map(|run| run.selected.waypoint),
            "name": state.run.as_ref().map(|run| run.name.as_str()),
            "result": state.result,
        })
    })
}
