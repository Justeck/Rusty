// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Ordinary local movement input, hotbar selection and crosshair attacks.
use super::Game;
use horizon_client::{collisions::SceneCollisions, items::ItemStack};
use horizon_client_render::mesher::{
    rusty, rusty_assistance as assist, rusty_freecam, rusty_utility as utility,
};
use horizon_core::geometry::{Aabb, Vec3};
use horizon_world::{
    collision_context::{CollisionContext, CollisionEntity, CollisionItem},
    entity_obstruction::EntityIdentity,
    nbt::Tag,
    world_movement::{CollisionCheckRequest, no_world_collision},
};
use std::{
    cell::RefCell,
    ops::ControlFlow,
    time::{Duration, Instant},
};

#[derive(Default)]
struct State {
    tool: assist::SelectionLease,
    last_attack_tick: Option<i64>,
    eat: Option<Eating>,
    eat_blocked: bool,
    movement: Option<MovementObservation>,
    attack_strength: f32,
    eat_reason: Option<&'static str>,
    eat_use_attempts: u32,
}
#[derive(Clone, Copy)]
struct MovementObservation {
    tick: i64,
    position: [f64; 3],
    on_ground: bool,
    fall_distance: f64,
    support_clear: Option<bool>,
    sneak: bool,
    jump: bool,
}
struct Eating {
    previous: usize,
    slot: usize,
    stack: ItemStack,
    count: u32,
    hunger: i32,
    started: Instant,
    observed_use: bool,
}
thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

fn allowed(game: &Game<'_>) -> bool {
    game.native_window()
        && game.native_on(rusty::ID)
        && game.native_playing()
        && game.was_playing
        && game.focused
        && !super::rusty_ui::is_open()
        && !rusty_freecam::active()
        && game.riding.is_none()
        && !game.spectator()
        && game
            .player
            .as_ref()
            .is_some_and(|p| !p.health.dead_or_dying() && !p.sleeping)
}

/// Bounded collision probes use the same contextual shapes as native physics.
/// Missing chunks or query failures are unknown, never reported as support.
fn clear(game: &Game<'_>, bounds: Aabb) -> Option<bool> {
    let min = bounds.min_position();
    let max = bounds.max_position();
    if [min.x, min.y, min.z, max.x, max.y, max.z]
        .iter()
        .any(|v| !v.is_finite())
    {
        return None;
    }
    if min.y < f64::from(game.world.bottom_y()) || max.y >= f64::from(game.world.top_y()) {
        return None;
    }
    let x0 = (min.x.floor() as i32) >> 4;
    let x1 = (max.x.floor() as i32) >> 4;
    let z0 = (min.z.floor() as i32) >> 4;
    let z1 = (max.z.floor() as i32) >> 4;
    if x1 - x0 > 1 || z1 - z0 > 1 {
        return None;
    }
    for x in x0..=x1 {
        for z in z0..=z1 {
            if !game.world.chunks.contains_key(&(x, z)) {
                return None;
            }
        }
    }
    let player = game.player.as_ref()?;
    let mut world = SceneCollisions {
        world: &game.world,
        binding: game.collision,
        subject: Some(player.powder_snow_subject()),
    };
    let context = CollisionContext::for_entity(
        CollisionEntity {
            identity: EntityIdentity(0),
            y: player.position.y,
            descending: player.shift_key_down,
            main_hand: Some(CollisionItem::EMPTY),
        },
        false,
    );
    no_world_collision(
        &mut world,
        CollisionCheckRequest {
            entity: EntityIdentity(0),
            context,
            bounds,
            candidate_budget: 256,
        },
    )
    .ok()
}

pub(super) fn movement(
    game: &Game<'_>,
    forward: &mut f32,
    strafe: &mut f32,
    jump: &mut bool,
    sneak: &mut bool,
) {
    if !allowed(game) {
        return;
    }
    let cfg = rusty::config();
    let settings = &cfg.assistance;
    if settings.validate().is_err() || (!settings.safe_walk && !settings.auto_jump) {
        return;
    }
    let Some(player) = game.player.as_ref() else {
        return;
    };
    if !player.ground.on_ground
        || player.flying
        || player.fall_flying
        || player.swimming
        || player.fluids.was_touching_water
        || player.is_in_lava()
        || game.using.is_some()
    {
        return;
    }
    let Some([dx, dz]) = assist::horizontal_direction(*forward, *strafe, game.yaw) else {
        return;
    };
    let bounds = player.bounds();
    let min = bounds.min_position();
    let max = bounds.max_position();
    // Look ahead at least a walking step, including ordinary sprint momentum.
    let distance = player
        .velocity
        .x
        .hypot(player.velocity.z)
        .max(0.35)
        .min(0.8);
    let (dx, dz) = (dx * distance, dz * distance);
    let support = Aabb::new(
        Vec3::new(min.x + dx + 1e-6, min.y - 0.6, min.z + dz + 1e-6),
        Vec3::new(max.x + dx - 1e-6, min.y, max.z + dz - 1e-6),
    );
    let support_clear = if settings.safe_walk && !*jump {
        clear(game, support)
    } else {
        None
    };
    if settings.safe_walk && !*jump && support_clear != Some(false) {
        *sneak = true;
    }
    if settings.auto_jump && !*sneak && !*jump {
        let obstruction = Aabb::new(
            Vec3::new(min.x + dx, min.y + 0.05, min.z + dz),
            Vec3::new(max.x + dx, min.y + 0.6, max.z + dz),
        );
        let headroom = Aabb::new(
            Vec3::new(min.x + dx, min.y + 1.001, min.z + dz),
            Vec3::new(max.x + dx, max.y + 1.001, max.z + dz),
        );
        if clear(game, obstruction) == Some(false) && clear(game, headroom) == Some(true) {
            *jump = true;
        }
    }
    STATE.with_borrow_mut(|state| {
        state.movement = Some(MovementObservation {
            tick: game.world.game_time,
            position: [player.position.x, player.position.y, player.position.z],
            on_ground: player.ground.on_ground,
            fall_distance: player.fall_distance,
            support_clear,
            sneak: *sneak,
            jump: *jump,
        })
    });
}

pub(super) fn status(_game: &Game<'_>) -> serde_json::Value {
    STATE.with_borrow(|state| serde_json::json!({
        "movement": state.movement.map(|m| serde_json::json!({"tick":m.tick,"position":m.position,"on_ground":m.on_ground,"fall_distance":m.fall_distance,"support_clear":m.support_clear,"requested_sneak":m.sneak,"requested_jump":m.jump})),
        "eating_owned": state.eat.as_ref().map(|e| e.slot+1), "eating_blocked":state.eat_blocked,
        "eating_reason":state.eat_reason,"eating_use_attempts":state.eat_use_attempts,
        "attack_strength": state.attack_strength,
    }))
}

pub(super) fn eating_paused() -> bool {
    STATE.with_borrow(|state| state.eat_blocked)
}

pub(super) fn busy() -> bool {
    STATE.with_borrow(|state| state.eat.is_some() || state.tool.owns_selection())
}

fn remaining(game: &Game<'_>, stack: &ItemStack) -> Option<Option<u8>> {
    let component = |id| stack.data().and_then(|data| data.other(id));
    if component("minecraft:unbreakable").is_some()
        || component("!minecraft:damage").is_some()
        || component("!minecraft:max_damage").is_some()
    {
        return Some(None);
    }
    let maximum = match component("minecraft:max_damage") {
        Some(Tag::Int(v)) => u32::try_from(*v).ok()?,
        Some(_) => return None,
        None => game.items().get(&stack.item)?.max_damage,
    };
    let observed = utility::Durability::observe(
        stack.damage,
        utility::DamageMaximum::Value(maximum),
        false,
        false,
    );
    match observed {
        utility::Durability::NotDamageable => Some(None),
        utility::Durability::Remaining { .. } => Some(observed.percentage()),
        _ => None,
    }
}

/// Selection only: no item movement, container clicks, mining or rotation.
pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    let cfg = rusty::config();
    let active = allowed(game)
        && cfg.assistance.validate().is_ok()
        && !super::rusty_automations::busy()
        && !super::rusty_builder::busy()
        && !super::rusty_inventory::busy();
    let tool_active = active
        && cfg.assistance.auto_tool
        && game.left
        && !game.right
        && game.using.is_none()
        && !game.creative;
    if tool_active {
        let desired = game.pick().and_then(|hit| {
            let facts = game
                .res
                .facts()
                .get(game.world.block(hit.pos[0], hit.pos[1], hit.pos[2]));
            if facts.hardness < 0. || facts.hardness == 0. {
                return None;
            }
            let tools: Vec<_> = game
                .inventory
                .iter()
                .take(9)
                .enumerate()
                .filter_map(|(slot, entry)| {
                    let stack = entry.as_ref().filter(|s| s.count > 0)?;
                    Some(assist::ToolCandidate {
                        slot,
                        speed: game.items().mining_speed(Some(stack), &facts.name),
                        correct_drops: game.items().correct_for_drops(Some(stack), &facts.name),
                        remaining_percent: remaining(game, stack)?,
                    })
                })
                .collect();
            assist::best_tool(&tools, game.selected, cfg.assistance.tool_min_remaining)
        });
        if let Some(slot) = STATE.with_borrow_mut(|state| state.tool.choose(game.selected, desired))
        {
            game.selected = slot;
        }
    } else if let Some(slot) = STATE.with_borrow_mut(|state| state.tool.stop(game.selected)) {
        game.selected = slot;
    }
    eat_frame(
        game,
        active && cfg.assistance.auto_eat,
        cfg.assistance.eat_below,
    );
    let attack_strength = if active && cfg.assistance.trigger_bot {
        if game.remote.is_some() {
            super::remote::rusty_assist_remote::attack_strength(game)
        } else {
            game.attack_strength_scale()
        }
    } else {
        0.
    };
    STATE.with_borrow_mut(|state| state.attack_strength = attack_strength);
    if active
        && cfg.assistance.trigger_bot
        && !game.left
        && !game.right
        && game.using.is_none()
        && attack_strength >= 1.0
    {
        let eligible = game.mob_hit().is_some_and(|(index, distance)| {
            let mob = &game.mobs[index];
            mob.health > 0.
                && !mob.removed
                && mob.death_time == 0
                && (cfg.assistance.trigger_players || mob.kind.id != "minecraft:player")
                && game.before_block(distance)
                && game.held_range_allows(distance)
        });
        if eligible && let Some(clock) = game.native_clock() {
            let fresh = STATE.with_borrow_mut(|state| {
                if state.last_attack_tick == Some(clock.game_time) {
                    false
                } else {
                    state.last_attack_tick = Some(clock.game_time);
                    true
                }
            });
            if fresh {
                if game.remote.is_some() {
                    super::remote::rusty_assist_remote::attack(game);
                } else {
                    game.attack();
                }
            }
        }
    }
}

fn user_use(game: &Game<'_>) -> bool {
    use winit::event::MouseButton;
    game.toggles.is_down(3)
        || game.physical_buttons.contains(&MouseButton::Right)
        || game.physical_keys.iter().any(|key| {
            game.settings
                .keys
                .translate_all(*key)
                .iter()
                .any(|mapped| matches!(mapped, Err(MouseButton::Right)))
        })
}

fn stop_eating(game: &mut Game<'_>, block: bool) {
    let owned = STATE.with_borrow_mut(|state| {
        state.eat_blocked = block;
        state.eat.take()
    });
    let Some(owned) = owned else {
        return;
    };
    let user = user_use(game);
    if !user {
        game.right = false;
        if game.remote.is_some() && game.using.is_some_and(|(slot, _)| slot == owned.slot) {
            super::remote::rusty_assist_remote::release_food(game);
        }
    }
    if game.selected == owned.slot && !user {
        game.selected = owned.previous;
    }
}

fn eat_frame(game: &mut Game<'_>, enabled: bool, threshold: u8) {
    let hunger = game.player.as_ref().map_or(20, |p| p.health.food.food);
    if !enabled || hunger > i32::from(threshold) {
        if !STATE.with_borrow(|s| s.eat.is_some()) {
            STATE.with_borrow_mut(|s| s.eat_blocked = false);
        }
    }
    let interrupted = !enabled || game.remote.is_none() || game.left || user_use(game);
    let progress = STATE.with_borrow_mut(|state| {
        state.eat.as_mut().map(|eat| {
            let stack = game.inventory.get(eat.slot).and_then(Option::as_ref);
            let same = stack.is_some_and(|stack| {
                assist::food_identity_matches(
                    (&eat.stack.item, eat.stack.damage, &eat.stack.extra),
                    (&stack.item, stack.damage, &stack.extra),
                )
            });
            let consumed = hunger > eat.hunger
                || stack.is_some_and(|s| same && s.count < eat.count)
                || (stack.is_none() && eat.observed_use);
            let using = game.using.is_some_and(|(slot, _)| slot == eat.slot);
            let progress = assist::eat_progress(
                eat.slot,
                game.selected,
                eat.observed_use,
                using,
                consumed,
                interrupted || (!same && !consumed) || !game.right,
                eat.started.elapsed() > Duration::from_secs(10),
            );
            let reason = if progress == assist::EatProgress::Cancel {
                Some(if game.selected != eat.slot {
                    "selection_changed"
                } else if !enabled {
                    "screen_focus_or_module_unavailable"
                } else if game.left || user_use(game) {
                    "user_input"
                } else if !same && !consumed {
                    "item_components_changed"
                } else if !game.right {
                    "use_request_released"
                } else if eat.started.elapsed() > Duration::from_secs(10) {
                    "timeout"
                } else {
                    "use_stopped"
                })
            } else {
                None
            };
            eat.observed_use |= using;
            (progress, reason)
        })
    });
    if let Some((_, Some(reason))) = progress {
        STATE.with_borrow_mut(|state| state.eat_reason = Some(reason));
    }
    match progress.map(|(progress, _)| progress) {
        Some(assist::EatProgress::Hold) => return,
        Some(assist::EatProgress::Complete) => {
            stop_eating(game, false);
            return;
        }
        Some(assist::EatProgress::Cancel) => {
            stop_eating(game, enabled);
            return;
        }
        None => {}
    }
    if interrupted
        // ClickGUI applies a queued close during native_frame, after the
        // engine's track_toggle_screens phase. Starting here in that same
        // frame would lose our new right-use request to the next transition's
        // releaseAll and falsely classify it as manual interruption.
        || !game.was_playing
        || game.right
        || game.using.is_some()
        || game.creative
        || hunger < 0
        || hunger > i32::from(threshold)
        || STATE.with_borrow(|state| state.eat_blocked)
    {
        return;
    }
    let foods: Vec<_> = game
        .inventory
        .iter()
        .take(9)
        .enumerate()
        .filter_map(|(slot, stack)| {
            let stack = stack.as_ref().filter(|s| s.count > 0)?;
            // Bound the copied identity snapshot before cloning any component
            // tree. A remote component patch is exact bytes, not compression.
            if stack.data().is_some_and(|data| {
                data.wire
                    .patch()
                    .is_none_or(|patch| patch.len() > 16 * 1024)
            }) {
                return None;
            }
            // Modified/removed food and consumable components remain unclassified.
            if stack.data().is_some_and(|data| {
                [
                    "minecraft:food",
                    "!minecraft:food",
                    "minecraft:consumable",
                    "!minecraft:consumable",
                ]
                .iter()
                .any(|id| data.other(id).is_some())
            }) {
                return None;
            }
            let info = game.items().get(&stack.item)?;
            let (nutrition, saturation) = info.food?;
            if info.consumable.as_ref().is_none_or(|c| c.ticks() <= 0) || info.equip_slot.is_some()
            {
                return None;
            }
            Some(assist::FoodCandidate {
                slot,
                id: &stack.item,
                nutrition,
                saturation,
            })
        })
        .collect();
    let Some(slot) = assist::best_food(&foods, game.selected) else {
        return;
    };
    let Some(stack) = game.inventory[slot].as_ref() else {
        return;
    };
    let eating = Eating {
        previous: game.selected,
        slot,
        stack: stack.clone(),
        count: stack.count,
        hunger,
        started: Instant::now(),
        observed_use: false,
    };
    STATE.with_borrow_mut(|state| {
        state.eat = Some(eating);
        state.eat_reason = None;
        state.eat_use_attempts = 0;
    });
    game.selected = slot;
    game.right = true;
}

/// Own automatic use goes straight to ordinary Item.use, so crosshair changes
/// cannot open containers, trade, mount, place or otherwise act on a target.
pub(super) fn use_item(game: &mut Game<'_>) -> ControlFlow<()> {
    let owned = STATE.with_borrow(|state| {
        state.eat.as_ref().map(|eat| {
            let valid = game
                .inventory
                .get(eat.slot)
                .and_then(Option::as_ref)
                .is_some_and(|stack| {
                    stack.count > 0
                        && assist::food_identity_matches(
                            (&eat.stack.item, eat.stack.damage, &eat.stack.extra),
                            (&stack.item, stack.damage, &stack.extra),
                        )
                });
            (eat.slot, valid)
        })
    });
    let Some((slot, valid)) = owned else {
        return ControlFlow::Continue(());
    };
    if !allowed(game)
        || !rusty::config().assistance.auto_eat
        || game.left
        || user_use(game)
        || game.selected != slot
        || !valid
    {
        stop_eating(game, true);
        // A physical user's explicit action retains the ordinary path.
        return if user_use(game) {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        };
    }
    STATE
        .with_borrow_mut(|state| state.eat_use_attempts = state.eat_use_attempts.saturating_add(1));
    super::remote::rusty_assist_remote::use_food(game);
    ControlFlow::Break(())
}

pub(super) fn reset(game: &mut Game<'_>) {
    // Host resets can run before frame reconciliation. Relinquish our own use
    // request before discarding ownership; never restore an old-world slot.
    if STATE.with_borrow(|state| state.eat.is_some()) && !user_use(game) {
        game.right = false;
    }
    STATE.with_borrow_mut(|state| *state = State::default());
}
