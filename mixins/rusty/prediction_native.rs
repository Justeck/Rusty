// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Read-only, deterministic centre-line estimates using Horizon's native ticks.
// Injected below `game::creatures` to reuse its private collision adapters.
use super::{Game, ProjWorld};
use horizon_client::{
    mobs::Who,
    projectiles::{Hit, Projectile, ProjectileKind, ProjectileWorld},
};
use horizon_client_render::mesher::rusty_prediction_core::known_sweep;
use horizon_core::{
    geometry::{Aabb, Vec3},
    random::LegacyRandom,
};

pub(in crate::game) struct Prediction {
    pub segments: Vec<[[f64; 3]; 2]>,
    pub hit: Option<[f64; 3]>,
    /// Full-draw previews are explicitly different from a currently charged bow.
    pub kind: &'static str,
    /// More target boxes were received than the bounded estimate can include.
    pub partial: bool,
    /// Read-only diagnostic status; an unknown corridor is not an impact.
    pub status: &'static str,
}
impl Prediction {
    fn empty() -> Self {
        Self {
            segments: Vec::new(),
            hit: None,
            kind: "",
            partial: false,
            status: "unsupported",
        }
    }
}

/// Delegate only queries. `ProjWorld::block_contacts` dispatches mod events;
/// deliberately inherit the trait's NOOP instead of forwarding that method.
struct Queries<'a, 'b, 'c>(&'a ProjWorld<'b, 'c>);
impl ProjectileWorld for Queries<'_, '_, '_> {
    fn clip(&self, from: Vec3, to: Vec3) -> Option<(Vec3, [i32; 3], [i32; 3])> {
        self.0.clip(from, to)
    }
    fn in_water(&self, bounds: Aabb) -> bool {
        self.0.in_water(bounds)
    }
    fn targets(&self) -> &[(Who, Aabb)] {
        self.0.targets()
    }
    fn empty(&self, p: [i32; 3]) -> bool {
        self.0.empty(p)
    }
    fn solid(&self, p: [i32; 3]) -> bool {
        self.0.solid(p)
    }
    fn ignites(&self, p: [i32; 3]) -> bool {
        self.0.ignites(p)
    }
}
fn xyz(v: Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}

/// Ordinary vanilla launch semantics only: no release/use handlers, world RNG,
/// inventory edits, impact handlers, damage, sound, or native mod dispatch.
/// Random launch spread is omitted intentionally, so this is an estimate rather
/// than a promise of the exact randomized shot or a moving entity's future.
pub(in crate::game) fn predict(game: &Game<'_>, max_steps: u32) -> Prediction {
    let mut out = Prediction::empty();
    let limit = max_steps.min(200);
    if limit == 0 {
        return out;
    }
    // Count source rows before any helper allocates or scans. Nine reserves
    // the player and the dragon's eight parts. The host helper then allocates
    // at most 4096 boxes and preserves its private special-target semantics.
    let rows = [
        game.mobs.len(),
        game.boats.len(),
        game.carts.len(),
        game.crystals.len(),
        9,
    ]
    .into_iter()
    .fold(0usize, usize::saturating_add);
    if rows > 4096 {
        out.partial = true;
        out.status = "entity scan limit";
        return out;
    }
    let Some(player) = game.player.as_ref() else {
        return out;
    };
    let Some(stack) = game.stack_in(game.selected) else {
        return out;
    };
    if stack.count == 0 {
        return out;
    }
    let (kind, speed, label) = match &*stack.item {
        "minecraft:bow" => {
            // projectile_slot is the immutable vanilla query. get_projectile
            // invokes mutable mod hooks and must never run during rendering.
            if !game.creative && game.projectile_slot(&stack.item).is_none() {
                return out;
            }
            let charge = game
                .using
                .filter(|(slot, _)| *slot == game.selected)
                .map(|(_, ticks)| ticks);
            let ticks = charge.unwrap_or(20).min(i32::MAX as u32) as i32;
            let Some(power) = crate::game::item_use::bow_release_power(true, ticks, false) else {
                return out;
            };
            (
                ProjectileKind::Arrow {
                    base_damage: 2.0,
                    critical: false,
                    effect: None,
                },
                power * 3.0,
                if charge.is_some() {
                    "bow"
                } else {
                    "bow full draw"
                },
            )
        }
        "minecraft:snowball" => (ProjectileKind::Snowball, 1.5, "snowball"),
        "minecraft:egg" | "minecraft:blue_egg" | "minecraft:brown_egg" => {
            (ProjectileKind::Egg, 1.5, "egg")
        }
        "minecraft:ender_pearl" => (ProjectileKind::EnderPearl, 1.5, "ender pearl"),
        _ => return out,
    };
    let Some(eye) = game.eye_position() else {
        return out;
    };
    let mut shadow = Projectile::new(
        kind,
        Vec3::new(eye[0], eye[1] - 0.1, eye[2]),
        Some(Who::Player),
    );
    if let Some(arrow) = shadow.arrow.as_mut() {
        arrow.set_owner(Some(Who::Player));
    }
    let mut local_random = LegacyRandom::new(0);
    shadow.shoot_from_rotation(
        &mut local_random,
        game.pitch,
        game.yaw,
        0.0,
        speed,
        0.0,
        game.known_player_movement(),
        player.ground.on_ground,
    );
    let base = game.mob_world_base(&[]);
    let mut targets = game.projectile_targets();
    targets.sort_unstable_by(|(who_a, a), (who_b, b)| {
        // Keep the owner box for the native left-owner immunity calculation.
        let distance = |who: &Who, bounds: &Aabb| {
            if *who == Who::Player {
                return -1.0;
            }
            let lo = bounds.min_position();
            let hi = bounds.max_position();
            let center = lo.add_vector(hi).scale(0.5);
            center.subtract(shadow.position).length_squared()
        };
        distance(who_a, a).total_cmp(&distance(who_b, b))
    });
    out.partial = targets.len() > 256;
    targets.truncate(256);
    let world = ProjWorld {
        w: &base,
        targets,
        border: Some(game.border.bounds()),
    };
    let queries = Queries(&world);
    let rain = game.arrow_rain();
    let scene = game.arrow_scene(&rain);
    out.kind = label;
    out.status = "step limit";
    out.segments.reserve(limit as usize);
    // Native arrow block effects also call block_contact::emit directly.
    // Remove that thread's authority with the host's panic-safe scoped guard.
    horizon_client::block_contact::in_level(None, || {
        for _ in 0..limit {
            let from = xyz(shadow.position);
            if !known_sweep(
                from,
                xyz(shadow.velocity),
                game.world.bottom_y(),
                game.world.top_y(),
                |x, z| game.world.chunks.contains_key(&(x, z)),
            ) {
                out.status = "unknown corridor or invalid motion";
                break;
            }
            let (alive, hit) = if shadow.arrow.is_some() {
                horizon_client::arrows::tick(&mut shadow, &scene, &world.targets)
            } else {
                shadow.tick(&queries)
            };
            let end = match hit {
                Hit::Block { at, .. } | Hit::Entity { at, .. } => xyz(at),
                Hit::None => xyz(shadow.position),
            };
            // Discard shadow-only feedback; never pass events to Game handlers.
            if let Some(arrow) = shadow.arrow.as_mut() {
                arrow.events.clear();
            }
            if end.iter().any(|v| !v.is_finite()) {
                out.status = "invalid native result";
                break;
            }
            out.segments.push([from, end]);
            if hit != Hit::None {
                out.hit = Some(end);
                out.status = "predicted impact";
                break;
            }
            if !alive || shadow.stuck {
                out.status = "stopped";
                break;
            }
        }
    });
    out
}
