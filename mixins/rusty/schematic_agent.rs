// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Own one normal host input task at a time. Never request an unvalidated path.
use super::{Game, Goto, Progress, TaskKind, navigation};
use std::sync::mpsc::{self, Receiver};
use winit::{event::MouseButton, keyboard::KeyCode};
pub(in crate::game) const NAME: &str = "rusty_schematic_builder";
pub(in crate::game) fn idle(game: &Game<'_>) -> bool {
    !game.agent_busy() && game.agent.glide.is_none() && game.flight.is_none()
}
pub(in crate::game) fn current(game: &Game<'_>) -> Option<&'static str> {
    game.agent.task.as_ref().map(|t| t.name)
}
pub(in crate::game) fn healthy(game: &Game<'_>) -> bool {
    game.agent.task.as_ref().is_none_or(|t| {
        t.name == NAME
            && match &t.kind {
                TaskKind::Goto(g) => {
                    g.replans == 0 && game.ticks.saturating_sub(g.progress_tick) < 40
                }
                _ => true,
            }
    })
}
pub(in crate::game) fn walk(
    game: &mut Game<'_>,
    from: [i32; 3],
    to: [i32; 3],
) -> Result<Receiver<String>, String> {
    if game.agent_busy() {
        return Err("Another agent owns the controls".into());
    }
    let p = game.player.as_ref().ok_or("No player")?.position;
    let position = [p.x, p.y, p.z];
    let target = [
        f64::from(to[0]) + 0.5,
        f64::from(to[1]),
        f64::from(to[2]) + 0.5,
    ];
    let goto = Goto {
        target,
        near: 0.15,
        sprint: false,
        path: navigation::Plan {
            nodes: vec![from, to],
            complete: true,
        },
        blocked: Vec::new(),
        progress: Progress::new(position, target),
        next: 1,
        progress_tick: game.ticks,
        replans: 0,
        jump_until: 0,
        warning: Some("Rusty checked schematic step".into()),
    };
    let (tx, rx) = mpsc::channel();
    game.start_task(NAME, TaskKind::Goto(Box::new(goto)), &tx, 4.)
        .map_err(|e| e.to_string())?;
    Ok(rx)
}
pub(in crate::game) fn cancel(game: &mut Game<'_>) {
    if current(game) != Some(NAME) {
        return;
    }
    let task = game.agent.task.take();
    for key in [KeyCode::KeyW, KeyCode::Space, KeyCode::ControlLeft] {
        let physical = game.physical_keys.iter().any(|raw| {
            game.settings
                .keys
                .translate_all(*raw)
                .iter()
                .any(|m| matches!(m,Ok(k) if *k==key))
        });
        let pad = game.settings.controller.enabled
            && match key {
                KeyCode::Space => game.pad.holds_action(&game.settings.controller, "key.jump"),
                KeyCode::ControlLeft => game
                    .pad
                    .holds_action(&game.settings.controller, "key.sprint"),
                _ => false,
            };
        if !physical && !pad && !(key == KeyCode::ControlLeft && game.toggles.is_down(1)) {
            game.keys.remove(&key);
        }
    }
    let attack = game.physical_buttons.contains(&MouseButton::Left)
        || game.physical_keys.iter().any(|raw| {
            game.settings
                .keys
                .translate_all(*raw)
                .iter()
                .any(|m| matches!(m, Err(MouseButton::Left)))
        })
        || (game.settings.controller.enabled
            && game
                .pad
                .holds_action(&game.settings.controller, "key.attack"));
    if !attack {
        let _ = game.agent_mouse(MouseButton::Left, false);
        game.mining = None;
    }
    if let Some(t) = task {
        let _ = t
            .reply
            .send(serde_json::json!({"error":"Schematic construction stopped"}).to_string());
    }
}

/// One bounded reachable hitpoint probe. The caller budgets four per tick.
pub(in crate::game) fn aim_candidate(
    game: &mut Game<'_>,
    target: [i32; 3],
    plan: &std::collections::BTreeMap<[i32; 3], u32>,
    attempt: usize,
) -> Option<Vec<([i32; 3], u32)>> {
    const DIRECTIONS: [[i32; 3]; 6] = [
        [0, -1, 0],
        [0, 0, -1],
        [0, 0, 1],
        [-1, 0, 0],
        [1, 0, 0],
        [0, 1, 0],
    ];
    const UV: [[f64; 2]; 5] = [
        [0.5, 0.5],
        [0.25, 0.25],
        [0.75, 0.75],
        [0.25, 0.75],
        [0.75, 0.25],
    ];
    let merging = attempt >= 30;
    let direction = *DIRECTIONS.get((attempt % 30) / 5)?;
    let support = if merging {
        target
    } else {
        std::array::from_fn(|i| target[i] + direction[i])
    };
    if support[1] < game.world.bottom_y()
        || support[1] >= game.world.top_y()
        || !game
            .world
            .chunks
            .contains_key(&(support[0] >> 4, support[2] >> 4))
        || crate::game::remote::rusty_build_actions::unsettled(game, support)
    {
        return None;
    }
    let f = game
        .res
        .facts()
        .get(game.world.block(support[0], support[1], support[2]));
    if merging {
        if !crate::game::remote::rusty_build_actions::partial(game, target, *plan.get(&target)?) {
            return None;
        }
    } else if !f.full_collision || f.replaceable || f.block_entity {
        return None;
    }
    let axis = direction.iter().position(|d| *d != 0)?;
    let face = direction.map(|v| -v);
    let uv = UV[attempt % 5];
    let mut point = support.map(|v| f64::from(v) + 0.5);
    point[axis] += f64::from(face[axis]) * 0.4999;
    point[(axis + 1) % 3] = f64::from(support[(axis + 1) % 3]) + uv[0];
    point[(axis + 2) % 3] = f64::from(support[(axis + 2) % 3]) + uv[1];
    let eye = game.eye_position()?;
    let (yaw, pitch) = super::yaw_pitch(eye, point);
    let hit = game.clip_ray(eye, crate::game::view_vector(yaw, pitch), game.reach())?;
    if hit.pos != support || hit.face != face {
        return None;
    }
    let old = (game.yaw, game.pitch);
    game.yaw = yaw;
    game.pitch = pitch;
    let writes = crate::game::remote::rusty_build_actions::probe_hit(game, target, plan, &hit).ok();
    if writes.is_none() {
        game.yaw = old.0;
        game.pitch = old.1;
    }
    writes
}
