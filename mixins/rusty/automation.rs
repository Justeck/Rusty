// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Independent bounded workflows over received observations and ordinary actions.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HookObservation {
    pub id: u32,
    pub biting: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FishAction {
    Reel,
    Cast,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum FishPhase {
    #[default]
    Idle,
    Watching {
        hook: u32,
        slot: usize,
    },
    Bite {
        hook: u32,
        slot: usize,
        since: i64,
    },
    Reeling {
        hook: u32,
        slot: usize,
        since: i64,
    },
    Recast {
        hook: u32,
        slot: usize,
        since: i64,
    },
    Casting {
        hook: u32,
        slot: usize,
        since: i64,
    },
}
#[derive(Default, Debug)]
pub struct Fishing {
    phase: FishPhase,
    blocked_hook: Option<u32>,
    last_tick: Option<i64>,
}
impl Fishing {
    pub fn dispatch_result(&mut self, hook: Option<HookObservation>, requested: bool) {
        if !requested {
            self.cancel(hook);
        }
    }
    #[allow(clippy::too_many_arguments)] // Complete, immutable observation for one bounded transition.
    pub fn step(
        &mut self,
        tick: i64,
        hook: Option<HookObservation>,
        slot: usize,
        usable: bool,
        manual: bool,
        delay: u8,
        recast: bool,
    ) -> Option<FishAction> {
        self.step_ready(tick, hook, slot, usable, manual, delay, recast, true)
    }
    pub fn cancel(&mut self, current: Option<HookObservation>) {
        self.phase = FishPhase::Idle;
        self.blocked_hook = current.map(|h| h.id);
    }
    pub fn phase(&self) -> &'static str {
        match self.phase {
            FishPhase::Idle => "waiting_for_user_cast",
            FishPhase::Watching { .. } => "watching",
            FishPhase::Bite { .. } => "bite_delay",
            FishPhase::Reeling { .. } => "awaiting_removal",
            FishPhase::Recast { .. } => "recast_delay",
            FishPhase::Casting { .. } => "awaiting_cast",
        }
    }
    #[allow(clippy::too_many_arguments)] // Input readiness is separate from eligibility/cancellation.
    pub fn step_ready(
        &mut self,
        tick: i64,
        hook: Option<HookObservation>,
        slot: usize,
        usable: bool,
        manual: bool,
        delay: u8,
        recast: bool,
        action_ready: bool,
    ) -> Option<FishAction> {
        if self.last_tick.is_some_and(|last| tick < last) {
            self.cancel(hook);
            self.last_tick = Some(tick);
            return None;
        }
        self.last_tick = Some(tick);
        let owned_slot = match self.phase {
            FishPhase::Idle => None,
            FishPhase::Watching { slot, .. }
            | FishPhase::Bite { slot, .. }
            | FishPhase::Reeling { slot, .. }
            | FishPhase::Recast { slot, .. }
            | FishPhase::Casting { slot, .. } => Some(slot),
        };
        if !usable
            || !(2..=20).contains(&delay)
            || slot >= 9
            || owned_slot.is_some_and(|owned| owned != slot)
        {
            self.cancel(hook);
            return None;
        }
        if manual {
            if owned_slot.is_some() {
                self.cancel(hook);
            }
            return None;
        }
        match self.phase {
            FishPhase::Idle => {
                if let Some(h) = hook.filter(|h| Some(h.id) != self.blocked_hook) {
                    self.phase = FishPhase::Watching { hook: h.id, slot };
                }
            }
            FishPhase::Watching { hook: id, slot } => match hook {
                Some(h) if h.id == id && h.biting => {
                    self.phase = FishPhase::Bite {
                        hook: id,
                        slot,
                        since: tick,
                    }
                }
                Some(h) if h.id != id => self.phase = FishPhase::Watching { hook: h.id, slot },
                None => self.cancel(None),
                _ => {}
            },
            FishPhase::Bite {
                hook: id,
                slot,
                since,
            } => match hook {
                Some(h) if h.id == id && !h.biting => {
                    self.phase = FishPhase::Watching { hook: id, slot }
                }
                Some(h) if h.id == id => {
                    if action_ready && tick.saturating_sub(since) >= i64::from(delay) {
                        self.phase = FishPhase::Reeling {
                            hook: id,
                            slot,
                            since: tick,
                        };
                        return Some(FishAction::Reel);
                    }
                }
                _ => self.cancel(hook),
            },
            FishPhase::Reeling {
                hook: id,
                slot,
                since,
            } => {
                if hook.is_none() {
                    self.phase = if recast {
                        FishPhase::Recast {
                            hook: id,
                            slot,
                            since: tick,
                        }
                    } else {
                        FishPhase::Idle
                    };
                } else if hook.is_some_and(|h| h.id != id) || tick.saturating_sub(since) > 40 {
                    self.cancel(hook);
                }
            }
            FishPhase::Recast {
                hook: id,
                slot,
                since,
            } => {
                if hook.is_some() {
                    self.cancel(hook);
                } else if !recast {
                    self.cancel(None);
                } else if action_ready && tick.saturating_sub(since) >= 20 {
                    self.phase = FishPhase::Casting {
                        hook: id,
                        slot,
                        since: tick,
                    };
                    return Some(FishAction::Cast);
                }
            }
            FishPhase::Casting {
                hook: id,
                slot,
                since,
            } => {
                if let Some(h) = hook.filter(|h| h.id != id) {
                    self.phase = FishPhase::Watching { hook: h.id, slot };
                } else if tick.saturating_sub(since) > 40 {
                    self.cancel(hook);
                }
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offhand {
    Empty,
    Totem,
    Other,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TotemAction {
    Swap { slot: usize },
    Restore { slot: usize },
}
#[derive(Debug, Default)]
pub struct Totem {
    pending: Option<(usize, usize, i64)>,
    blocked: Option<u64>,
}
impl Totem {
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    #[allow(clippy::too_many_arguments)] // One snapshot; the state machine performs no I/O.
    pub fn step(
        &mut self,
        tick: i64,
        selected: usize,
        candidate: Option<usize>,
        offhand: Offhand,
        signature: u64,
        usable: bool,
        manual: bool,
    ) -> Option<TotemAction> {
        if let Some((previous, owned, since)) = self.pending {
            if selected != owned {
                self.pending = None;
                self.blocked = Some(signature);
                return None;
            }
            let complete = offhand == Offhand::Totem;
            if complete
                || !usable
                || manual
                || offhand == Offhand::Other
                || tick < since
                || tick.saturating_sub(since) > 40
            {
                self.pending = None;
                self.blocked = if complete { None } else { Some(signature) };
                return (selected != previous).then_some(TotemAction::Restore { slot: previous });
            }
            return None;
        }
        if !usable
            || manual
            || offhand != Offhand::Empty
            || selected >= 9
            || self.blocked == Some(signature)
        {
            return None;
        }
        let slot = candidate.filter(|slot| *slot < 9)?;
        self.blocked = None;
        self.pending = Some((selected, slot, tick));
        Some(TotemAction::Swap { slot })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hook(id: u32, biting: bool) -> Option<HookObservation> {
        Some(HookObservation { id, biting })
    }
    #[test]
    fn fishing_never_casts_without_an_observed_user_hook_and_waits_real_bite_delay() {
        let mut f = Fishing::default();
        for tick in 0..100 {
            assert_eq!(f.step(tick, None, 0, true, false, 4, true), None);
        }
        assert_eq!(f.step(100, hook(7, false), 0, true, true, 4, true), None);
        assert_eq!(f.step(101, hook(7, false), 0, true, false, 4, true), None);
        assert_eq!(f.phase(), "watching");
        assert_eq!(f.step(102, hook(7, true), 0, true, false, 4, true), None);
        assert_eq!(f.step(105, hook(7, true), 0, true, false, 4, true), None);
        assert_eq!(
            f.step(106, hook(7, true), 0, true, false, 4, true),
            Some(FishAction::Reel)
        );
        assert_eq!(f.step(106, hook(7, true), 0, true, false, 4, true), None);
    }
    #[test]
    fn fishing_recast_requires_removal_and_is_one_request_until_new_hook_arrives() {
        let mut f = Fishing::default();
        f.step(0, hook(7, false), 2, true, false, 2, true);
        f.step(1, hook(7, true), 2, true, false, 2, true);
        assert_eq!(
            f.step(3, hook(7, true), 2, true, false, 2, true),
            Some(FishAction::Reel)
        );
        for tick in 4..10 {
            assert_eq!(f.step(tick, hook(7, true), 2, true, false, 2, true), None);
        }
        assert_eq!(f.step(10, None, 2, true, false, 2, true), None);
        assert_eq!(f.step(29, None, 2, true, false, 2, true), None);
        assert_eq!(
            f.step(30, None, 2, true, false, 2, true),
            Some(FishAction::Cast)
        );
        for tick in 31..35 {
            assert_eq!(f.step(tick, None, 2, true, false, 2, true), None);
        }
        f.step(35, hook(8, false), 2, true, false, 2, true);
        assert_eq!(f.phase(), "watching");
    }
    #[test]
    fn fishing_cancels_on_slot_screen_manual_and_lost_bite_without_repeated_actions() {
        let mut f = Fishing::default();
        f.step(0, hook(1, false), 0, true, false, 2, true);
        f.step(1, hook(1, true), 0, true, false, 2, true);
        assert_eq!(f.step(2, hook(1, false), 0, true, false, 2, true), None);
        assert_eq!(f.phase(), "watching");
        f.step(3, hook(1, true), 0, true, false, 2, true);
        assert_eq!(f.step(5, hook(1, true), 1, true, false, 2, true), None);
        assert_eq!(f.phase(), "waiting_for_user_cast");
        assert_eq!(f.step(8, hook(1, true), 0, true, false, 2, true), None);
        f.step(9, hook(2, false), 0, true, false, 2, true);
        f.step(10, hook(2, true), 0, false, false, 2, true);
        assert_eq!(f.phase(), "waiting_for_user_cast");
        f.step(11, hook(3, false), 0, true, false, 2, true);
        assert_eq!(f.step(12, hook(3, true), 0, true, true, 2, true), None);
        assert_eq!(f.phase(), "waiting_for_user_cast");
    }
    #[test]
    fn fishing_pending_timeout_and_clock_rollback_never_retry_or_cast() {
        let mut f = Fishing::default();
        f.step(100, hook(1, false), 0, true, false, 2, true);
        f.step(101, hook(1, true), 0, true, false, 2, true);
        assert_eq!(
            f.step(103, hook(1, true), 0, true, false, 2, true),
            Some(FishAction::Reel)
        );
        assert_eq!(f.step(144, hook(1, true), 0, true, false, 2, true), None);
        assert_eq!(f.phase(), "waiting_for_user_cast");
        assert_eq!(f.step(200, hook(1, true), 0, true, false, 2, true), None);
        assert_eq!(f.step(0, hook(2, true), 0, true, false, 2, true), None);
        assert_eq!(f.phase(), "waiting_for_user_cast");
    }
    #[test]
    fn fishing_waits_native_use_cooldown_without_forgetting_observed_bite() {
        let mut f = Fishing::default();
        f.step_ready(0, hook(1, false), 0, true, false, 2, true, true);
        f.step_ready(1, hook(1, true), 0, true, false, 2, true, false);
        assert_eq!(
            f.step_ready(3, hook(1, true), 0, true, false, 2, true, false),
            None
        );
        assert_eq!(f.phase(), "bite_delay");
        assert_eq!(
            f.step_ready(4, hook(1, true), 0, true, false, 2, true, true),
            Some(FishAction::Reel)
        );
    }
    #[test]
    fn refused_fishing_dispatch_stops_without_claiming_a_reel_or_retrying() {
        let mut f = Fishing::default();
        f.step(0, hook(1, false), 0, true, false, 2, true);
        f.step(1, hook(1, true), 0, true, false, 2, true);
        assert_eq!(
            f.step(3, hook(1, true), 0, true, false, 2, true),
            Some(FishAction::Reel)
        );
        f.dispatch_result(hook(1, true), false);
        assert_eq!(f.phase(), "waiting_for_user_cast");
        assert_eq!(f.step(20, hook(1, true), 0, true, false, 2, true), None);
    }
    #[test]
    fn totem_waits_for_authoritative_offhand_before_restoring_its_selection() {
        let mut t = Totem::default();
        assert_eq!(
            t.step(1, 2, Some(5), Offhand::Empty, 1, true, false),
            Some(TotemAction::Swap { slot: 5 })
        );
        assert!(t.pending());
        assert_eq!(t.step(2, 5, Some(5), Offhand::Empty, 1, true, false), None);
        assert_eq!(
            t.step(3, 5, None, Offhand::Totem, 2, true, false),
            Some(TotemAction::Restore { slot: 2 })
        );
        assert!(!t.pending());
        assert_eq!(t.step(4, 2, None, Offhand::Totem, 2, true, false), None);
    }
    #[test]
    fn totem_protects_other_items_and_manual_selection_and_does_not_retry_timeout() {
        let mut t = Totem::default();
        assert_eq!(t.step(1, 0, Some(5), Offhand::Other, 1, true, false), None);
        assert_eq!(
            t.step(2, 0, Some(5), Offhand::Empty, 2, true, false),
            Some(TotemAction::Swap { slot: 5 })
        );
        assert_eq!(t.step(3, 4, Some(5), Offhand::Empty, 2, true, false), None);
        assert_eq!(t.step(4, 4, Some(5), Offhand::Empty, 2, true, false), None);
        assert_eq!(
            t.step(5, 4, Some(5), Offhand::Empty, 3, true, false),
            Some(TotemAction::Swap { slot: 5 })
        );
        assert_eq!(
            t.step(46, 5, Some(5), Offhand::Empty, 3, true, false),
            Some(TotemAction::Restore { slot: 4 })
        );
        assert_eq!(t.step(80, 4, Some(5), Offhand::Empty, 3, true, false), None);
    }
}
