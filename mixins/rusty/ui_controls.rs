// Pure value mapping shared by native menu sliders and the root test harness.
#[derive(Clone, Copy, Debug)]
pub struct Range {
    pub min: f32,
    pub max: f32,
    pub step: f32,
}
impl Range {
    fn snap(self, value: f32) -> f32 {
        let value = if value.is_finite() { value } else { self.min };
        (self.min + ((value.clamp(self.min, self.max) - self.min) / self.step).round() * self.step)
            .clamp(self.min, self.max)
    }
    pub fn value(self, fraction: f32) -> f32 {
        let fraction = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if fraction == 1.0 {
            return self.max;
        }
        self.snap(self.min + fraction * (self.max - self.min))
    }
    pub fn fraction(self, value: f32) -> f32 {
        let value = if value.is_finite() { value } else { self.min };
        ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }
    pub fn nudge(self, value: f32, steps: f32) -> f32 {
        let value = if value.is_finite() { value } else { self.min };
        let steps = if steps.is_finite() { steps } else { 0.0 };
        self.snap(value + steps * self.step)
    }
}

/// A menu consumes held controls. Repeated events cannot restore those controls
/// until a release or a genuinely new press; analog sticks must return neutral.
#[derive(Default, Debug)]
pub struct FreshPressGate {
    blocked: std::collections::BTreeSet<String>,
    movement_needs_center: bool,
    look_needs_center: bool,
}
impl FreshPressGate {
    pub fn consume<'a>(&mut self, names: impl IntoIterator<Item = &'a str>) {
        for name in names {
            if self.blocked.len() >= 256 {
                break;
            }
            if !name.is_empty() && name.len() <= 96 {
                self.blocked.insert(name.to_owned());
            }
        }
        self.movement_needs_center = true;
        self.look_needs_center = true;
    }
    pub fn blocked(&self, name: &str) -> bool {
        self.blocked.contains(name)
    }
    pub fn key(&mut self, name: &str, pressed: bool, repeat: bool) -> bool {
        if !pressed || !repeat {
            self.blocked.remove(name);
            false
        } else {
            self.blocked(name)
        }
    }
    pub fn movement(
        &mut self,
        value: Option<[f32; 2]>,
        observed_centered: bool,
    ) -> Option<[f32; 2]> {
        if value.is_some_and(|v| !v.into_iter().all(f32::is_finite)) {
            return None;
        }
        if self.movement_needs_center {
            if observed_centered {
                self.movement_needs_center = false;
            }
            None
        } else {
            value
        }
    }
    pub fn look(&mut self, centered: bool) -> bool {
        if self.look_needs_center && centered {
            self.look_needs_center = false;
        }
        !self.look_needs_center
    }
}

#[cfg(test)]
mod tests {
    use super::{FreshPressGate, Range};
    const PERCENT: Range = Range {
        min: 0.0,
        max: 100.0,
        step: 1.0,
    };
    #[test]
    fn endpoints_and_pointer_overflow_are_bounded() {
        assert_eq!(PERCENT.value(-1.0), 0.0);
        assert_eq!(PERCENT.value(1.0), 100.0);
        assert_eq!(PERCENT.value(2.0), 100.0);
        assert_eq!(PERCENT.fraction(25.0), 0.25);
    }
    #[test]
    fn drag_and_buttons_use_the_same_steps() {
        let width = Range {
            min: 0.5,
            max: 4.0,
            step: 0.1,
        };
        assert!((width.value(0.2) - 1.2).abs() < 0.0001);
        assert!((width.nudge(1.2, 1.0) - 1.3).abs() < 0.0001);
        assert_eq!(width.nudge(4.0, 1.0), 4.0);
        assert_eq!(width.nudge(0.5, -1.0), 0.5);
    }
    #[test]
    fn fine_speed_steps_reach_both_endpoints() {
        let speed = Range {
            min: 0.05,
            max: 8.0,
            step: 0.05,
        };
        assert_eq!(speed.value(0.0), 0.05);
        assert_eq!(speed.value(1.0), 8.0);
        assert!((speed.nudge(1.0, -1.0) - 0.95).abs() < 0.0001);
    }
    #[test]
    fn non_finite_input_never_reaches_a_config() {
        assert_eq!(PERCENT.value(f32::NAN), 0.0);
        assert!(PERCENT.fraction(f32::INFINITY).is_finite());
        assert!(PERCENT.nudge(f32::NAN, 1.0).is_finite());
    }
    #[test]
    fn menu_consumed_keys_ignore_repeats_until_release_or_a_fresh_press() {
        let mut gate = FreshPressGate::default();
        gate.consume(["key.keyboard.w", "key.keyboard.q"]);
        assert!(gate.key("key.keyboard.w", true, true));
        assert!(gate.blocked("key.keyboard.q"));
        assert!(!gate.key("key.keyboard.w", false, false));
        assert!(!gate.key("key.keyboard.w", true, true));
        assert!(!gate.key("key.keyboard.q", true, false));
        assert!(!gate.blocked("key.keyboard.q"));
        assert!(!gate.key("key.keyboard.a", true, false));
    }
    #[test]
    fn analog_input_requires_neutral_after_a_menu_and_nan_never_clears_it() {
        let mut gate = FreshPressGate::default();
        gate.consume(["key.keyboard.w"]);
        assert_eq!(gate.movement(Some([0., 1.]), false), None);
        assert_eq!(gate.movement(Some([f32::NAN, 0.]), true), None);
        // None while unfocused or disabled is unavailable, not proof of neutral.
        assert_eq!(gate.movement(None, false), None);
        assert_eq!(gate.movement(Some([0., 1.]), false), None);
        assert_eq!(gate.movement(None, true), None);
        assert_eq!(gate.movement(Some([0., 1.]), false), Some([0., 1.]));
        assert!(!gate.look(false));
        assert!(gate.look(true));
        assert!(gate.look(false));
    }
}
