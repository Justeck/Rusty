// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
/// Aggregation reuse never suppresses chunk invalidation or bounded scanning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub position: [u64; 3],
    pub config: u64,
    pub context: bool,
}
#[derive(Default)]
pub struct Gate {
    last: Option<Stamp>,
}
impl Gate {
    pub fn aggregate(&mut self, stamp: Stamp, changed: bool) -> bool {
        if stamp
            .position
            .into_iter()
            .any(|bits| !f64::from_bits(bits).is_finite())
        {
            self.clear();
            return true;
        }
        let aggregate = changed || self.last != Some(stamp);
        self.last = Some(stamp);
        aggregate
    }
    pub fn clear(&mut self) {
        self.last = None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stationary_results_reuse_but_all_material_changes_invalidate() {
        let mut g = Gate::default();
        let s = Stamp {
            position: [1., 2., 3.].map(f64::to_bits),
            config: 4,
            context: true,
        };
        assert!(g.aggregate(s, false));
        assert!(!g.aggregate(s, false));
        assert!(g.aggregate(s, true));
        assert!(!g.aggregate(s, false));
        let moved = Stamp {
            position: [1., 2.1, 3.].map(f64::to_bits),
            ..s
        };
        assert!(g.aggregate(moved, false));
        let configured = Stamp { config: 5, ..moved };
        assert!(g.aggregate(configured, false));
        let context = Stamp {
            context: false,
            ..configured
        };
        assert!(g.aggregate(context, false));
        g.clear();
        assert!(g.aggregate(context, false));
    }
    #[test]
    fn non_finite_positions_are_never_reused() {
        let mut g = Gate::default();
        let s = Stamp {
            position: [f64::NAN, 0., 0.].map(f64::to_bits),
            config: 1,
            context: false,
        };
        assert!(g.aggregate(s, false));
        assert!(g.aggregate(s, false));
    }
}
