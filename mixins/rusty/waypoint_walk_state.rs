// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Pure session guards for explicitly started waypoint walking.
#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    pub identity: String,
    pub dimension: String,
    pub waypoint: u64,
    pub position: [f64; 3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Unavailable,
    Focus,
    Menu,
    Freecam,
    Manual,
    Selection,
    Replaced,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Idle,
    Keep,
    Cancel(StopReason),
    Relinquish,
}
pub struct Context<'a> {
    pub available: bool,
    pub focused: bool,
    pub playing: bool,
    pub freecam: bool,
    pub manual: bool,
    pub selection: Option<&'a Selection>,
    pub task_name: Option<&'a str>,
}
pub const TASK_NAME: &str = "rusty_waypoint_walk";
pub fn decision(active: Option<&Selection>, context: &Context<'_>) -> Decision {
    let Some(active) = active else {
        return Decision::Idle;
    };
    // Ownership wins over every cancellation trigger: never stop someone else's task.
    if context.task_name != Some(TASK_NAME) {
        return Decision::Relinquish;
    }
    let why = if !context.available {
        Some(StopReason::Unavailable)
    } else if !context.focused {
        Some(StopReason::Focus)
    } else if !context.playing {
        Some(StopReason::Menu)
    } else if context.freecam {
        Some(StopReason::Freecam)
    } else if context.manual {
        Some(StopReason::Manual)
    } else if context.selection != Some(active) {
        Some(StopReason::Selection)
    } else {
        None
    };
    why.map_or(Decision::Keep, Decision::Cancel)
}
pub fn release_owned_input<const N: usize>(
    current: [bool; N],
    physical: [bool; N],
    owned: [bool; N],
) -> [bool; N] {
    std::array::from_fn(|i| if owned[i] { physical[i] } else { current[i] })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn selection(world: &str, dimension: &str) -> Selection {
        Selection {
            identity: world.into(),
            dimension: dimension.into(),
            waypoint: 1,
            position: [4.5, 64., 8.5],
        }
    }
    fn context(s: &Selection) -> Context<'_> {
        Context {
            available: true,
            focused: true,
            playing: true,
            freecam: false,
            manual: false,
            selection: Some(s),
            task_name: Some(TASK_NAME),
        }
    }
    #[test]
    fn interruption_cancels_only_our_running_task() {
        let s = selection("world-a", "minecraft:overworld");
        for why in 0..5 {
            let mut c = context(&s);
            let expected = match why {
                0 => {
                    c.available = false;
                    StopReason::Unavailable
                }
                1 => {
                    c.focused = false;
                    StopReason::Focus
                }
                2 => {
                    c.playing = false;
                    StopReason::Menu
                }
                3 => {
                    c.freecam = true;
                    StopReason::Freecam
                }
                _ => {
                    c.manual = true;
                    StopReason::Manual
                }
            };
            assert_eq!(decision(Some(&s), &c), Decision::Cancel(expected));
            c.task_name = Some("break");
            assert_eq!(decision(Some(&s), &c), Decision::Relinquish);
        }
    }
    #[test]
    fn identical_waypoint_numbers_in_another_world_or_dimension_do_not_continue() {
        let s = selection("world-a", "minecraft:overworld");
        for next in [
            selection("world-b", "minecraft:overworld"),
            selection("world-a", "minecraft:the_nether"),
        ] {
            assert_eq!(
                decision(Some(&s), &context(&next)),
                Decision::Cancel(StopReason::Selection)
            );
        }
        assert_eq!(decision(Some(&s), &context(&s)), Decision::Keep);
    }
    #[test]
    fn changed_or_deleted_selection_cancels_without_retargeting() {
        let s = selection("world-a", "minecraft:overworld");
        let mut next = s.clone();
        next.position[0] += 1.;
        assert_eq!(
            decision(Some(&s), &context(&next)),
            Decision::Cancel(StopReason::Selection)
        );
        let mut c = context(&s);
        c.selection = None;
        assert_eq!(
            decision(Some(&s), &c),
            Decision::Cancel(StopReason::Selection)
        );
    }
    #[test]
    fn completed_or_foreign_task_is_relinquished_and_never_auto_restarted() {
        let s = selection("world-a", "minecraft:overworld");
        let mut c = context(&s);
        c.task_name = None;
        assert_eq!(decision(Some(&s), &c), Decision::Relinquish);
        c.task_name = Some("goto");
        assert_eq!(decision(Some(&s), &c), Decision::Relinquish);
        assert_eq!(decision(None, &c), Decision::Idle);
    }
    #[test]
    fn cancellation_preserves_real_forward_and_jump_input() {
        assert_eq!(
            release_owned_input(
                [true, true, false, true],
                [false, false, true, false],
                [true, false, false, true]
            ),
            [false, true, false, false]
        );
        // Route owns W/Space/Ctrl; a physical W/Space press takes over from it.
        assert_eq!(
            release_owned_input([true, true, true], [true, true, false], [true; 3]),
            [true, true, false]
        );
        assert_eq!(
            release_owned_input([true, false, false], [false, false, false], [true; 3]),
            [false, false, false]
        );
        assert_eq!(
            release_owned_input([false, false, false], [false, false, true], [true; 3]),
            [false, false, true]
        );
    }
}
