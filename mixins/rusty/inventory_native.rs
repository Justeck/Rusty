// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Conservative player-inventory actions; the ordinary host predicts and corrects clicks.
use super::Game;
use horizon_client::items::ItemStack;
use horizon_client_render::mesher::{rusty, rusty_freecam, rusty_inventory_core as core};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

struct Pending {
    target: usize,
    expected: ItemStack,
    started: Instant,
}
#[derive(Default)]
struct State {
    memory: core::RefillMemory<ItemStack>,
    dirty: [bool; 9],
    dirty_armor: [bool; 4],
    armed: bool,
    armor_attempted: core::GearAttempts,
    manual_intents: core::ManualIntents,
    refill_enabled: bool,
    selected: Option<usize>,
    pending: Option<Pending>,
    quiet: Option<Instant>,
    last_action: Option<&'static str>,
}
thread_local! {static STATE:RefCell<State>=RefCell::new(State::default());}
fn bounded(stack: &ItemStack) -> bool {
    stack.count > 0
        && stack.data().is_none_or(|data| {
            data.wire
                .patch()
                .is_some_and(|patch| patch.len() <= 16 * 1024)
        })
}
fn snapshot(game: &Game<'_>) -> [Option<ItemStack>; 9] {
    std::array::from_fn(|slot| {
        game.inventory[slot]
            .as_ref()
            .filter(|stack| bounded(stack))
            .cloned()
    })
}
pub(super) fn mark(slot: usize) {
    STATE.with_borrow_mut(|state| {
        if slot < 9 {
            state.dirty[slot] = true;
        } else if (36..40).contains(&slot) {
            state.dirty_armor[3 - (slot - 36)] = true;
        }
    });
}
pub(super) fn mark_menu(container: i32, slot: i32) {
    if container == 0 {
        if (36..45).contains(&slot) {
            mark((slot - 36) as usize);
        } else if (5..9).contains(&slot) {
            STATE.with_borrow_mut(|state| state.dirty_armor[(slot - 5) as usize] = true);
        }
    }
}
/// Called only after an incoming inventory packet has assigned its slots.
pub(super) fn received(game: &Game<'_>) {
    let now = now(game);
    let eligible = allowed(game);
    STATE.with_borrow_mut(|state| {
        let dirty = std::mem::take(&mut state.dirty);
        let dirty_armor = std::mem::take(&mut state.dirty_armor);
        if !state.armed {
            return;
        }
        for (part, dirty) in dirty_armor.into_iter().enumerate() {
            if dirty {
                state
                    .armor_attempted
                    .received(part, game.armor[part].is_some());
            }
        }
        for (slot, dirty) in dirty.into_iter().enumerate() {
            if dirty && state.refill_enabled {
                let stack = game.inventory[slot]
                    .as_ref()
                    .filter(|stack| bounded(stack))
                    .cloned();
                // An empty packet is not classified as success or rejection.
                // Fresh ordinary input permits at most one new refill either way.
                let fresh = state.manual_intents.take(slot, now);
                if stack.is_none()
                    && fresh
                    && eligible
                    && game.selected == slot
                    && state.pending.is_none()
                {
                    state.memory.renew_user_intent(slot);
                }
                state.memory.received(slot, stack);
            }
        }
        if state.pending.as_ref().is_some_and(|pending| {
            let changed = if pending.target < 9 {
                dirty[pending.target]
            } else {
                dirty_armor[pending.target - 500]
            };
            let stack = if pending.target < 9 {
                game.inventory[pending.target].as_ref()
            } else {
                game.armor
                    .get(pending.target - 500)
                    .and_then(Option::as_ref)
            };
            changed && stack.is_some_and(|stack| stack.same_item_same_components(&pending.expected))
        }) {
            state.pending = None;
            state.last_action = Some("received_target");
            state.quiet = Some(Instant::now());
        }
    });
}
fn allowed(game: &Game<'_>) -> bool {
    game.native_window()
        && game.native_on(rusty::ID)
        && game.native_playing()
        && game.was_playing
        && game.remote.is_some()
        && game.focused
        && !super::rusty_ui::is_open()
        && !rusty_freecam::active()
        && !game.spectator()
        && !game.creative
        && !game.agent_busy()
        && game.riding.is_none()
        && game.carried.is_none()
        && game.remote_menu().0 == 0
        && game
            .player
            .as_ref()
            .is_some_and(|p| !p.sleeping && !p.health.dead_or_dying())
}
fn now(game: &Game<'_>) -> u64 {
    u64::try_from(game.started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// A fresh configured USE/ATTACK edge is renewed intent, not an acknowledgment.
/// Called after the control layer's existing context/fresh-press gate.
pub(super) fn input(game: &Game<'_>, key: &str, pressed: bool) {
    if pressed
        && [
            "key.hotbar.1",
            "key.hotbar.2",
            "key.hotbar.3",
            "key.hotbar.4",
            "key.hotbar.5",
            "key.hotbar.6",
            "key.hotbar.7",
            "key.hotbar.8",
            "key.hotbar.9",
        ]
        .iter()
        .any(|mapping| key == game.settings.keys.get(mapping))
    {
        cancel_intents();
    }
    let action = if key == game.settings.keys.get("key.use") {
        0
    } else if key == game.settings.keys.get("key.attack") {
        1
    } else {
        return;
    };
    intent_edge(game, action, pressed);
}

/// Controller mappings are semantic actions, independent of keyboard bindings.
/// Pad releases reach this path even after a menu consumes their gameplay use.
pub(super) fn action_input(game: &Game<'_>, action: &str, pressed: bool) {
    if !game.native_window() || !game.native_on(rusty::ID) {
        return;
    }
    if pressed
        && matches!(
            action,
            horizon_client::gamepad::HOTBAR_PREVIOUS | horizon_client::gamepad::HOTBAR_NEXT
        )
    {
        cancel_intents();
    }
    let action = match action {
        "key.use" => 0,
        "key.attack" => 1,
        _ => return,
    };
    intent_edge(game, action, pressed);
}

fn intent_edge(game: &Game<'_>, action: usize, pressed: bool) {
    let eligible = allowed(game)
        && rusty::config().assistance.refill
        && game.using.is_none()
        && game.hotbar_presses.is_empty()
        && !super::rusty_assist::busy()
        && !super::rusty_automations::busy()
        && !super::rusty_builder::busy();
    STATE.with_borrow_mut(|state| {
        let exact = state
            .memory
            .blocked_template(game.selected)
            .is_some_and(|template| {
                game.inventory
                    .get(game.selected)
                    .and_then(Option::as_ref)
                    .is_some_and(|stack| {
                        bounded(stack) && stack.same_item_same_components(template)
                    })
            });
        let eligible =
            eligible && state.armed && state.refill_enabled && state.pending.is_none() && exact;
        if state
            .manual_intents
            .edge(action, pressed, game.selected, now(game), eligible)
        {
            state.last_action = Some("fresh_ordinary_input");
            state.quiet = Some(Instant::now());
        }
    });
}
pub(super) fn cancel_intents() {
    STATE.with_borrow_mut(|state| state.manual_intents.cancel());
}
fn manual(game: &Game<'_>) -> bool {
    game.left
        || game.right
        || game.using.is_some()
        || !game.physical_buttons.is_empty()
        || !game.physical_keys.is_empty()
}
pub(super) fn busy() -> bool {
    STATE.with_borrow(|state| state.pending.is_some())
}
pub(super) fn reset() {
    STATE.with_borrow_mut(|state| *state = State::default());
}
pub(super) fn status() -> serde_json::Value {
    STATE.with_borrow(|state|serde_json::json!({"pending":state.pending.is_some(),"last_action":state.last_action}))
}
fn gear(game: &Game<'_>) -> Vec<core::GearCandidate> {
    game.inventory
        .iter()
        .take(36)
        .enumerate()
        .filter_map(|(source, stack)| {
            let stack = stack.as_ref().filter(|s| bounded(s))?;
            if stack.data().is_some_and(|data| {
                data.enchantments
                    .iter()
                    .any(|(name, level)| &**name == "minecraft:binding_curse" && *level > 0)
                    || [
                        "minecraft:equippable",
                        "!minecraft:equippable",
                        "minecraft:attribute_modifiers",
                        "!minecraft:attribute_modifiers",
                    ]
                    .iter()
                    .any(|key| data.other(key).is_some())
            }) {
                return None;
            }
            let info = game.items().get(&stack.item)?;
            let part = match info.equip_slot.as_deref()? {
                "head" => 0,
                "chest" => 1,
                "legs" => 2,
                "feet" => 3,
                _ => return None,
            };
            if info.max_damage == 0 || stack.damage >= info.max_damage {
                return None;
            }
            let remaining = ((u64::from(info.max_damage - stack.damage) * 100)
                / u64::from(info.max_damage))
            .max(1) as u8;
            let mut armor = 0.;
            let mut toughness = 0.;
            for (attribute, _, amount, operation, _) in &info.modifiers {
                if operation == "add_value" {
                    match attribute.as_str() {
                        "minecraft:armor" => armor += amount,
                        "minecraft:armor_toughness" => toughness += amount,
                        _ => {}
                    }
                }
            }
            Some(core::GearCandidate {
                source,
                part,
                armor,
                toughness,
                remaining,
            })
        })
        .collect()
}
pub(super) fn frame(game: &mut Game<'_>) {
    let configuration = rusty::config();
    let cfg = &configuration.assistance;
    STATE.with_borrow_mut(|state| {
        if state.refill_enabled != cfg.refill {
            state.refill_enabled = cfg.refill;
            state.manual_intents.reset_requests();
            state.memory.restart(if cfg.refill && allowed(game) {
                snapshot(game)
            } else {
                std::array::from_fn(|_| None)
            });
        }
        if state.selected != Some(game.selected) {
            state.manual_intents.cancel();
            state.selected = Some(game.selected);
        }
    });
    if !allowed(game) || (!cfg.refill && !cfg.armor_assist) {
        STATE.with_borrow_mut(|state| {
            state.memory.suspend();
            state.armed = false;
            state.pending = None;
            state.quiet = Some(Instant::now());
            state.armor_attempted = core::GearAttempts::default();
            state.manual_intents.cancel();
        });
        return;
    }
    if manual(game) {
        STATE.with_borrow_mut(|state| {
            state.quiet = Some(Instant::now());
        });
        return;
    }
    let ready = STATE.with_borrow_mut(|state| {
        if !state.armed {
            if cfg.refill {
                state.memory.baseline(snapshot(game));
            }
            state.armed = true;
            state.quiet = Some(Instant::now());
            return false;
        }
        if let Some(pending) = &state.pending {
            if pending.started.elapsed() < Duration::from_millis(750) {
                return false;
            }
            state.pending = None;
            state.last_action = Some("prediction_wait_complete");
            state.quiet = Some(Instant::now());
            return false;
        }
        state
            .quiet
            .is_none_or(|time| time.elapsed() >= Duration::from_millis(500))
    });
    if !ready
        || super::rusty_assist::busy()
        || super::rusty_automations::busy()
        || super::rusty_builder::busy()
    {
        return;
    }
    let armor = if cfg.armor_assist {
        let empty = std::array::from_fn(|part| {
            game.armor[part].is_none()
                && STATE.with_borrow(|state| state.armor_attempted.available(part))
        });
        if empty.iter().any(|empty| *empty) {
            core::choose_gear(&gear(game), empty)
        } else {
            None
        }
    } else {
        None
    };
    let action = if let Some((source, part)) = armor {
        STATE.with_borrow_mut(|state| state.armor_attempted.request(part));
        Some((source, 500 + part, None))
    } else if cfg.refill {
        (0..9).find_map(|target| {
            if game.inventory[target].is_some() {
                return None;
            }
            let source = STATE.with_borrow(|state| {
                let wanted = state.memory.demand(target)?;
                (9..36).find(|source| {
                    game.inventory[*source].as_ref().is_some_and(|stack| {
                        bounded(stack) && stack.same_item_same_components(wanted)
                    })
                })
            })?;
            STATE.with_borrow_mut(|state| {
                state.memory.request(target);
            });
            Some((source, target, Some(target)))
        })
    } else {
        None
    };
    let Some((source, target, swap)) = action else {
        return;
    };
    let Some(expected) = game.inventory[source]
        .as_ref()
        .filter(|s| bounded(s))
        .cloned()
    else {
        return;
    };
    let requested = super::remote::rusty_inventory_actions::click(game, source, swap);
    STATE.with_borrow_mut(|state| {
        state.quiet = Some(Instant::now());
        state.last_action = Some(if requested {
            if swap.is_some() {
                "refill_requested"
            } else {
                "armor_requested"
            }
        } else {
            "request_refused"
        });
        if requested {
            if swap.is_some() {
                state.manual_intents.requested(target, now(game));
            }
            state.pending = Some(Pending {
                target,
                expected,
                started: Instant::now(),
            });
        }
    });
}
