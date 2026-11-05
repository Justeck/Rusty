// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Child of game::agent: reuse the host's bounded loaded-terrain A* task.
use super::{Game, Goto, Progress, TaskKind};
use std::sync::mpsc::{self, Receiver};
use winit::keyboard::KeyCode;

const NAME: &str = "rusty_waypoint_walk";
const OWNED_KEYS: [KeyCode; 3] = [KeyCode::KeyW, KeyCode::Space, KeyCode::ControlLeft];

pub(in crate::game) fn current(game: &Game<'_>) -> Option<&'static str> {
    game.agent.task.as_ref().map(|task| task.name)
}

pub(in crate::game) fn start(
    game: &mut Game<'_>,
    target: [f64; 3],
) -> Result<Receiver<String>, String> {
    game.playing().map_err(|error| error.to_string())?;
    if game.agent_busy() || game.agent.glide.is_some() || game.flight.is_some() {
        return Err("Finish the current movement or agent action first".into());
    }
    if target
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 29_999_984.)
    {
        return Err("The selected waypoint has invalid coordinates".into());
    }
    let p = game
        .player
        .as_ref()
        .ok_or("No player is available")?
        .position;
    let position = [p.x, p.y, p.z];
    let from = [p.x.floor() as i32, p.y.round() as i32, p.z.floor() as i32];
    let near = 0.65;
    let path = game
        .find_path(from, target, near, &[])
        .ok_or("No walking path through the currently loaded terrain")?;
    let mut next = 1;
    let aim = path.aim(position, &mut next, target).unwrap_or(target);
    let goto = Goto {
        target,
        near,
        sprint: false,
        path,
        blocked: Vec::new(),
        progress: Progress::new(position, aim),
        next,
        progress_tick: game.ticks,
        replans: 0,
        jump_until: 0,
        warning: None,
    };
    // The host task sends exactly one terminal reply. Only this receiver is retained.
    let (sender, receiver) = mpsc::channel();
    game.start_task(NAME, TaskKind::Goto(Box::new(goto)), &sender, 120.)
        .map_err(|error| error.to_string())?;
    Ok(receiver)
}

/// Cancel our task without releasing another agent's task, mouse buttons or
/// user-owned movement. The host's generic stop releases all of those.
pub(in crate::game) fn cancel(game: &mut Game<'_>) -> bool {
    if current(game) != Some(NAME) {
        return false;
    }
    let Some(task) = game.agent.task.take() else {
        return false;
    };
    let current = OWNED_KEYS.map(|key| game.keys.contains(&key));
    let physical = OWNED_KEYS.map(|key| {
        let action = match key {
            KeyCode::Space => Some("key.jump"),
            KeyCode::ControlLeft => Some("key.sprint"),
            _ => None,
        };
        let pad = game.settings.controller.enabled
            && action
                .is_some_and(|action| game.pad.holds_action(&game.settings.controller, action));
        let toggled_sprint = key == KeyCode::ControlLeft && game.toggles.is_down(1);
        pad || toggled_sprint
            || game.physical_keys.iter().any(|raw| {
                game.settings
                    .keys
                    .translate_all(*raw)
                    .iter()
                    .any(|mapped| matches!(mapped, Ok(mapped) if *mapped == key))
            })
    });
    let keep =
        super::super::rusty_waypoint_walk_state::release_owned_input(current, physical, [true; 3]);
    for (key, held) in OWNED_KEYS.into_iter().zip(keep) {
        if held {
            game.keys.insert(key);
        } else {
            game.keys.remove(&key);
        }
    }
    let _ = task.reply.send(
        serde_json::json!({"error":"Waypoint walking cancelled","arrived":false}).to_string(),
    );
    true
}
