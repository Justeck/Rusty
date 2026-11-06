// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Own one normal host input task at a time. Never request an unvalidated path.
use super::{Break, Game, Goto, Progress, TaskKind, navigation};
use std::{
    collections::BTreeMap,
    sync::mpsc::{self, Receiver},
};
use winit::{event::MouseButton, keyboard::KeyCode};
pub(in crate::game) const NAME: &str = "rusty_auto_mine";
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
        warning: Some("Rusty checked mining step".into()),
    };
    let (tx, rx) = mpsc::channel();
    game.start_task(NAME, TaskKind::Goto(Box::new(goto)), &tx, 4.)
        .map_err(|e| e.to_string())?;
    Ok(rx)
}
pub(in crate::game) fn mine(
    game: &mut Game<'_>,
    pos: [i32; 3],
    seconds: f64,
) -> Result<Receiver<String>, String> {
    if game.agent_busy() {
        return Err("Another agent owns the controls".into());
    }
    game.playing().map_err(|e| e.to_string())?;
    game.aim_block(pos, None).map_err(|e| e.to_string())?;
    if game.pick_entity_hit().is_some() {
        return Err("An entity blocks the mining line".into());
    }
    let task = Break {
        pos,
        before: game.world.block(pos[0], pos[1], pos[2]),
        items_before: game.dropped.iter().map(|i| i.id).collect(),
        inventory: game.inventory_counts(),
        started_tick: game.ticks,
        collect_until: None,
        broke_tick: 0,
        drops: BTreeMap::new(),
    };
    let (tx, rx) = mpsc::channel();
    game.start_task(NAME, TaskKind::Break(Box::new(task)), &tx, seconds)
        .map_err(|e| e.to_string())?;
    // The ordinary task presses attack only after the next safety gate.
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
            .send(serde_json::json!({"error":"Auto Mine stopped"}).to_string());
    }
}
