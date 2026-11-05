// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Independent inventory intent tracking; native adapters own ordinary actions.

pub struct RefillMemory<T> {
    templates: [Option<T>; 9],
    demand: [bool; 9],
    blocked: [bool; 9],
}
impl<T> Default for RefillMemory<T> {
    fn default() -> Self {
        Self {
            templates: std::array::from_fn(|_| None),
            demand: [false; 9],
            blocked: [false; 9],
        }
    }
}
impl<T: Clone> RefillMemory<T> {
    pub fn baseline(&mut self, slots: [Option<T>; 9]) {
        for (slot, stack) in slots.into_iter().enumerate() {
            if !self.blocked[slot] {
                self.templates[slot] = stack;
                self.demand[slot] = false;
            }
        }
    }
    pub fn received(&mut self, slot: usize, stack: Option<T>) {
        if slot >= 9 {
            return;
        }
        if let Some(stack) = stack {
            self.templates[slot] = Some(stack);
            self.demand[slot] = false;
            self.blocked[slot] = false;
        } else if !self.blocked[slot] && self.templates[slot].is_some() {
            self.demand[slot] = true;
        }
    }
    pub fn demand(&self, slot: usize) -> Option<&T> {
        (slot < 9 && self.demand[slot] && !self.blocked[slot])
            .then(|| self.templates[slot].as_ref())
            .flatten()
    }
    pub fn request(&mut self, slot: usize) -> Option<T> {
        let stack = self.demand(slot)?.clone();
        self.demand[slot] = false;
        self.blocked[slot] = true;
        Some(stack)
    }
    pub fn suspend(&mut self) {
        for (slot, template) in self.templates.iter_mut().enumerate() {
            if !self.blocked[slot] {
                *template = None;
            }
        }
        self.demand = [false; 9];
    }
    pub fn blocked_template(&self, slot: usize) -> Option<&T> {
        (slot < 9 && self.blocked[slot])
            .then(|| self.templates[slot].as_ref())
            .flatten()
    }
    pub fn renew_user_intent(&mut self, slot: usize) -> bool {
        if self.blocked_template(slot).is_none() {
            return false;
        }
        self.blocked[slot] = false;
        self.demand[slot] = false;
        true
    }
    pub fn restart(&mut self, slots: [Option<T>; 9]) {
        *self = Self::default();
        self.baseline(slots);
    }
}

/// An ordinary fresh press renews intent; it is never server acknowledgment.
#[derive(Default)]
pub struct ManualIntents {
    held: [bool; 2],
    requested: [Option<u64>; 9],
    granted: [Option<u64>; 9],
}
impl ManualIntents {
    pub fn requested(&mut self, slot: usize, now: u64) {
        if slot < 9 {
            self.requested[slot] = Some(now);
            self.granted[slot] = None;
        }
    }
    pub fn edge(
        &mut self,
        action: usize,
        pressed: bool,
        slot: usize,
        now: u64,
        eligible: bool,
    ) -> bool {
        let Some(held) = self.held.get_mut(action) else {
            return false;
        };
        let rising = pressed && !*held;
        *held = pressed;
        if !rising
            || !eligible
            || slot >= 9
            || !self.requested[slot]
                .and_then(|then| now.checked_sub(then))
                .is_some_and(|elapsed| elapsed >= 750)
        {
            return false;
        }
        self.granted[slot] = Some(now);
        true
    }
    pub fn take(&mut self, slot: usize, now: u64) -> bool {
        slot < 9
            && self.granted[slot]
                .take()
                .and_then(|then| now.checked_sub(then))
                .is_some_and(|elapsed| elapsed <= 10_000)
    }
    pub fn cancel(&mut self) {
        self.granted = [None; 9];
    }
    pub fn reset_requests(&mut self) {
        self.cancel();
        self.requested = [None; 9];
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GearCandidate {
    pub source: usize,
    pub part: usize,
    pub armor: f64,
    pub toughness: f64,
    pub remaining: u8,
}

#[derive(Default)]
pub struct GearAttempts([bool; 4]);
impl GearAttempts {
    pub fn available(&self, part: usize) -> bool {
        self.0.get(part).is_some_and(|blocked| !blocked)
    }
    pub fn request(&mut self, part: usize) {
        if let Some(blocked) = self.0.get_mut(part) {
            *blocked = true;
        }
    }
    pub fn received(&mut self, part: usize, nonempty: bool) {
        if nonempty && let Some(blocked) = self.0.get_mut(part) {
            *blocked = false;
        }
    }
}
pub fn choose_gear(candidates: &[GearCandidate], empty: [bool; 4]) -> Option<(usize, usize)> {
    for (part, empty) in empty.into_iter().enumerate() {
        if !empty {
            continue;
        }
        let best = candidates
            .iter()
            .filter(|g| {
                g.source < 36
                    && g.part == part
                    && g.armor.is_finite()
                    && g.armor > 0.
                    && g.armor <= 64.
                    && g.toughness.is_finite()
                    && g.toughness >= 0.
                    && g.toughness <= 32.
                    && g.remaining > 0
                    && g.remaining <= 100
            })
            .max_by(|a, b| {
                a.armor
                    .total_cmp(&b.armor)
                    .then(a.toughness.total_cmp(&b.toughness))
                    .then(a.remaining.cmp(&b.remaining))
                    .then(b.source.cmp(&a.source))
            });
        if let Some(best) = best {
            return Some((best.source, part));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Item {
        id: &'static str,
        components: &'static str,
    }
    fn stone() -> Item {
        Item {
            id: "minecraft:stone",
            components: "plain",
        }
    }
    #[test]
    fn refill_never_invents_an_intent_for_an_initial_empty_slot() {
        let mut memory = RefillMemory::<Item>::default();
        memory.baseline(std::array::from_fn(|_| None));
        memory.received(2, None);
        assert!(memory.demand(2).is_none());
        let mut slots = std::array::from_fn(|_| None);
        slots[2] = Some(stone());
        memory.baseline(slots);
        memory.received(2, None);
        assert_eq!(memory.demand(2), Some(&stone()));
    }
    #[test]
    fn refill_consumes_the_request_and_only_incoming_positive_state_rearms_it() {
        let mut memory = RefillMemory::default();
        memory.received(0, Some(stone()));
        memory.received(0, None);
        assert_eq!(memory.request(0), Some(stone()));
        // A predicted positive inventory snapshot cannot rearm this slot.
        let mut predicted = std::array::from_fn(|_| None);
        predicted[0] = Some(stone());
        memory.baseline(predicted);
        memory.received(0, None);
        assert!(memory.request(0).is_none());
        memory.received(0, Some(stone()));
        memory.received(0, None);
        assert_eq!(memory.request(0), Some(stone()));
        assert!(memory.request(0).is_none());
    }
    #[test]
    fn menu_suspension_discards_old_demands_and_components_remain_exact() {
        let mut memory = RefillMemory::default();
        let named = Item {
            id: "minecraft:stone",
            components: "named_private_stack",
        };
        memory.received(3, Some(named.clone()));
        memory.received(3, None);
        assert_eq!(memory.demand(3), Some(&named));
        memory.suspend();
        assert!(memory.demand(3).is_none());
        memory.baseline(std::array::from_fn(|_| None));
        assert!(memory.request(3).is_none());
    }
    fn gear(
        source: usize,
        part: usize,
        armor: f64,
        toughness: f64,
        remaining: u8,
    ) -> GearCandidate {
        GearCandidate {
            source,
            part,
            armor,
            toughness,
            remaining,
        }
    }
    #[test]
    fn armor_only_fills_empty_parts_and_uses_protection_before_durability() {
        let candidates = [
            gear(9, 0, 2., 0., 100),
            gear(10, 0, 3., 2., 10),
            gear(11, 1, 8., 2., 100),
        ];
        assert_eq!(
            choose_gear(&candidates, [true, false, false, false]),
            Some((10, 0))
        );
        assert_eq!(
            choose_gear(&candidates, [false, true, false, false]),
            Some((11, 1))
        );
        assert_eq!(choose_gear(&candidates, [false; 4]), None);
    }
    #[test]
    fn armor_rejects_invalid_or_broken_candidates_and_has_stable_ties() {
        let candidates = [
            gear(40, 0, 30., 0., 100),
            gear(0, 4, 30., 0., 100),
            gear(9, 0, f64::NAN, 0., 100),
            gear(10, 0, 5., 0., 0),
            gear(12, 0, 2., 0., 50),
            gear(11, 0, 2., 0., 50),
        ];
        assert_eq!(choose_gear(&candidates, [true; 4]), Some((11, 0)));
    }

    #[test]
    fn armor_rearms_only_after_a_received_nonempty_equipment_update() {
        let mut attempts = GearAttempts::default();
        assert!(attempts.available(0));
        attempts.request(0);
        assert!(!attempts.available(0));
        attempts.received(0, false);
        assert!(!attempts.available(0));
        // A rejected empty-slot correction cannot turn into repeated clicks.
        attempts.received(1, true);
        assert!(!attempts.available(0));
        attempts.received(0, true);
        assert!(attempts.available(0));
        // Later break/removal leaves this fresh, server-observed opportunity.
        attempts.received(0, false);
        assert!(attempts.available(0));
        attempts.request(0);
        assert!(!attempts.available(0));
    }

    #[test]
    fn fresh_use_after_no_echo_refill_permits_one_new_empty_request() {
        let mut memory = RefillMemory::default();
        let mut inputs = ManualIntents::default();
        memory.received(0, Some(stone()));
        memory.received(0, None);
        memory.request(0);
        inputs.requested(0, 100);
        assert_eq!(memory.blocked_template(0), Some(&stone()));
        // No echo: matching client prediction alone grants nothing.
        memory.received(0, None);
        assert!(memory.request(0).is_none());
        assert!(!inputs.edge(0, true, 0, 849, true));
        inputs.edge(0, false, 0, 849, true);
        assert!(inputs.edge(0, true, 0, 850, true));
        assert!(inputs.take(0, 900));
        assert!(memory.renew_user_intent(0));
        memory.received(0, None);
        assert_eq!(memory.request(0), Some(stone()));
        assert!(memory.request(0).is_none());
        assert!(!inputs.take(0, 901));
    }

    #[test]
    fn held_repeated_and_menu_crossing_input_never_renews_without_release() {
        let mut inputs = ManualIntents::default();
        inputs.requested(0, 0);
        assert!(!inputs.edge(0, true, 0, 800, false));
        inputs.cancel();
        assert!(!inputs.edge(0, true, 0, 900, true));
        inputs.edge(0, false, 0, 901, true);
        assert!(inputs.edge(0, true, 0, 902, true));
        assert!(!inputs.edge(0, true, 0, 903, true));
        assert!(inputs.take(0, 904));
        assert!(!inputs.take(0, 905));
    }

    #[test]
    fn attack_renews_once_but_stale_cancelled_clock_rollback_and_disabled_intents_do_not() {
        let mut inputs = ManualIntents::default();
        inputs.requested(1, 100);
        assert!(!inputs.edge(1, true, 1, 50, true));
        inputs.edge(1, false, 1, 60, true);
        assert!(inputs.edge(1, true, 1, 900, true));
        assert!(!inputs.take(1, 10_901));
        inputs.edge(1, false, 1, 11_000, true);
        assert!(inputs.edge(1, true, 1, 11_001, true));
        inputs.cancel();
        assert!(!inputs.take(1, 11_002));
        inputs.edge(1, false, 1, 12_000, true);
        inputs.reset_requests();
        assert!(!inputs.edge(1, true, 1, 12_001, true));
    }

    #[test]
    fn explicit_refill_reenable_uses_a_new_baseline_and_preserves_initially_empty_slots() {
        let mut memory = RefillMemory::default();
        memory.received(0, Some(stone()));
        memory.received(0, None);
        memory.request(0);
        // Explicit off/on creates a new controller, unlike a menu suspension.
        let mut baseline = std::array::from_fn(|_| None);
        baseline[0] = Some(stone());
        memory.restart(baseline);
        memory.received(1, None);
        assert!(memory.request(1).is_none());
        memory.received(0, None);
        assert_eq!(memory.request(0), Some(stone()));
    }

    #[test]
    fn suspension_preserves_blocked_identity_without_passive_rearming() {
        let mut memory = RefillMemory::default();
        let mut input = ManualIntents::default();
        memory.received(0, Some(stone()));
        memory.received(0, None);
        memory.request(0);
        input.requested(0, 0);
        memory.suspend();
        input.cancel();
        let mut resumed = std::array::from_fn(|_| None);
        resumed[0] = Some(stone());
        memory.baseline(resumed);
        assert_eq!(memory.blocked_template(0), Some(&stone()));
        memory.received(0, None);
        assert!(memory.request(0).is_none());
        assert!(input.edge(0, true, 0, 800, true));
        assert!(input.take(0, 850));
        assert!(memory.renew_user_intent(0));
        memory.received(0, None);
        assert_eq!(memory.request(0), Some(stone()));
        assert!(memory.request(0).is_none());
        assert!(memory.blocked_template(1).is_none());
    }
}
