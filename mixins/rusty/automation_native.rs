// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Received-hook fishing and an acknowledged ordinary empty-offhand swap.
use super::Game;
use horizon_client_render::mesher::{rusty, rusty_automation as core, rusty_freecam};
use std::{
    cell::RefCell,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::{Duration, Instant},
};

#[derive(Default)]
struct State {
    fish: core::Fishing,
    totem: core::Totem,
    swap_started: Option<Instant>,
    last_hook: Option<core::HookObservation>,
    last_bite: Option<(u32, i64)>,
    last_action: Option<&'static str>,
}
thread_local! {static STATE:RefCell<State>=RefCell::new(State::default());}

fn allowed(game: &Game<'_>) -> bool {
    if super::rusty_mining_native::active() { return false; }
    game.native_window()
        && game.native_on(rusty::ID)
        && game.native_playing()
        && game.was_playing
        && game.remote.is_some()
        && game.focused
        && !super::rusty_ui::is_open()
        && !rusty_freecam::active()
        && !game.spectator()
        && game.riding.is_none()
        && game
            .player
            .as_ref()
            .is_some_and(|p| !p.sleeping && !p.health.dead_or_dying())
}
fn manual(game: &Game<'_>) -> bool {
    use winit::keyboard::KeyCode;
    game.left
        || game.right
        || game.using.is_some()
        || !game.physical_buttons.is_empty()
        || game.physical_keys.iter().any(|key| {
            game.settings.keys.translate_all(*key).iter().any(|mapped| {
                matches!(
                    mapped,
                    Ok(KeyCode::Digit1
                        | KeyCode::Digit2
                        | KeyCode::Digit3
                        | KeyCode::Digit4
                        | KeyCode::Digit5
                        | KeyCode::Digit6
                        | KeyCode::Digit7
                        | KeyCode::Digit8
                        | KeyCode::Digit9
                        | KeyCode::KeyF
                        | KeyCode::KeyQ)
                )
            })
        })
}
fn inventory_signature(game: &Game<'_>) -> u64 {
    let mut hash = DefaultHasher::new();
    for stack in game
        .inventory
        .iter()
        .take(9)
        .chain(std::iter::once(&game.offhand))
    {
        if let Some(stack) = stack {
            for character in stack.item.chars().take(128) {
                character.hash(&mut hash);
            }
            stack.count.hash(&mut hash);
            stack.damage.hash(&mut hash);
        } else {
            0u8.hash(&mut hash);
        }
    }
    hash.finish()
}
fn normal_totem(stack: &horizon_client::items::ItemStack) -> bool {
    stack.count > 0
        && &*stack.item == "minecraft:totem_of_undying"
        && !stack.data().is_some_and(|data| {
            data.other("minecraft:death_protection").is_some()
                || data.other("!minecraft:death_protection").is_some()
        })
}
fn normal_rod(game: &Game<'_>) -> bool {
    game.inventory
        .get(game.selected)
        .and_then(Option::as_ref)
        .is_some_and(|stack| {
            stack.count > 0
                && &*stack.item == "minecraft:fishing_rod"
                && !stack.data().is_some_and(|data| {
                    data.other("minecraft:consumable").is_some()
                        || data.other("!minecraft:consumable").is_some()
                })
                && game.items().get(&stack.item).is_some_and(|info| {
                    stack.damage < info.max_damage
                        || stack
                            .data()
                            .is_some_and(|data| data.other("minecraft:unbreakable").is_some())
                })
        })
}

/// The shared client should suppress new automatic item/attack requests while
/// a slot-changing offhand action is awaiting its server-provided hand state.
pub(super) fn busy() -> bool {
    STATE.with_borrow(|state| state.totem.pending())
}

pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    let cfg = rusty::config();
    let ready = allowed(game)
        && cfg.assistance.validate().is_ok()
        && !super::rusty_builder::busy()
        && !super::rusty_inventory::busy();
    let manual = manual(game);
    // These are input delays, so use the monotonic client tick counter rather
    // than a server's adjustable world clock (/time and time synchronisation).
    let tick = i64::try_from(game.ticks).unwrap_or(i64::MAX);
    let hook = super::fishing::rusty_automation_fishing::own_hook(game);
    let offhand = match game
        .offhand
        .as_ref()
        .filter(|s| s.count > 0 && &*s.item != "minecraft:air")
    {
        None => core::Offhand::Empty,
        Some(stack) if normal_totem(stack) => core::Offhand::Totem,
        Some(_) => core::Offhand::Other,
    };
    let candidate = game
        .inventory
        .get(game.selected)
        .and_then(Option::as_ref)
        .filter(|s| normal_totem(s))
        .map(|_| game.selected)
        .or_else(|| {
            game.inventory
                .iter()
                .take(9)
                .position(|stack| stack.as_ref().is_some_and(normal_totem))
        });
    let signature = inventory_signature(game);
    let expired = STATE.with_borrow(|state| {
        state
            .swap_started
            .is_some_and(|at| at.elapsed() > Duration::from_secs(5))
    });
    let totem_action = STATE.with_borrow_mut(|state| {
        state.last_hook = hook;
        if let Some(hook) = hook.filter(|hook| hook.biting) {
            state.last_bite = Some((hook.id, tick));
        }
        let action = state.totem.step(
            tick,
            game.selected,
            candidate,
            offhand,
            signature,
            ready && cfg.assistance.auto_totem,
            manual || expired,
        );
        if !state.totem.pending() {
            state.swap_started = None;
            if !cfg.assistance.auto_totem {
                state.totem = core::Totem::default();
            }
        }
        action
    });
    match totem_action {
        Some(core::TotemAction::Swap { slot }) => {
            // No menu clicks, no predicted inventory mutation. The normal
            // native swap sends both server hand updates back to this client.
            game.selected = slot;
            super::remote::rusty_automation_actions::swap_offhand(game);
            STATE.with_borrow_mut(|state| {
                state.swap_started = Some(Instant::now());
                state.last_action = Some("offhand_swap_requested");
            });
        }
        Some(core::TotemAction::Restore { slot }) => {
            game.selected = slot;
            STATE.with_borrow_mut(|state| state.last_action = Some("selection_released"));
        }
        None => {}
    }
    let fish_action = STATE.with_borrow_mut(|state| {
        if !cfg.assistance.auto_fish {
            state.fish = core::Fishing::default();
            return None;
        }
        // A higher-priority automatic hand swap pauses observation without
        // revoking the user's cast. Explicit user actions still cancel below.
        if state.totem.pending() && !manual {
            return None;
        }
        let usable = ready && !state.totem.pending() && normal_rod(game);
        state.fish.step_ready(
            tick,
            hook,
            game.selected,
            usable,
            manual,
            cfg.assistance.fish_delay,
            cfg.assistance.fish_recast,
            game.use_delay == 0 && !game.hands_busy() && game.using.is_none(),
        )
    });
    if let Some(action) = fish_action {
        // The finite state machine emits at most one ordinary action, then
        // waits for received hook removal/new-hook metadata before continuing.
        let requested = super::remote::rusty_automation_actions::use_main(game);
        STATE.with_borrow_mut(|state| {
            state.fish.dispatch_result(hook, requested);
            state.last_action = Some(match action {
                core::FishAction::Reel if requested => "reel_requested",
                core::FishAction::Cast if requested => "recast_requested",
                core::FishAction::Reel => "reel_refused",
                core::FishAction::Cast => "recast_refused",
            })
        });
    }
}

pub(super) fn status() -> serde_json::Value {
    STATE.with_borrow(|state|serde_json::json!({"fish_phase":state.fish.phase(),"own_hook":state.last_hook.map(|h|serde_json::json!({"id":h.id,"biting":h.biting})),"last_bite":state.last_bite.map(|(id,tick)|serde_json::json!({"id":id,"client_tick":tick})),"totem_pending":state.totem.pending(),"last_action":state.last_action}))
}
pub(super) fn reset() {
    STATE.with_borrow_mut(|state| *state = State::default());
}
