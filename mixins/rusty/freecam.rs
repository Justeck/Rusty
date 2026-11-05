// Copyright 2026 umbx-dot, MIT; see LICENSE-Camera.rs.
// Modified 2026-11-03 for Rusty by Justeck: integrated settings, safe input lifecycle and swept collision.
use std::cell::RefCell;

pub const ID: &str = "rusty_camera";
pub const VERSION: &str = "1.0.0";
pub const DESCRIPTION: &str = "Fly the camera independently of your player's controls. WASD, Space and Shift move it, Ctrl goes faster and the mouse wheel changes the speed. Your body still follows normal game physics.";
pub const KEY: &str = "toggle";
pub const DEFAULT_KEY: &str = "key.keyboard.u";
pub const SPEEDS: [f64; 6] = [0.25, 0.5, 1.0, 1.5, 2.5, 4.0];
pub const FORWARD: usize = 0;
pub const BACK: usize = 1;
pub const LEFT: usize = 2;
pub const RIGHT: usize = 3;
pub const UP: usize = 4;
pub const DOWN: usize = 5;
pub const SPRINT: usize = 6;
const DEFAULT_SPEED: f64 = 1.0;
const MIN_SPEED: f64 = 0.05;
const MAX_SPEED: f64 = 8.0;
const WHEEL_STEP: f64 = 1.2;
const SPRINT_FACTOR: f64 = 2.0;
const TURN: f64 = 0.15;
const MAX_PITCH: f32 = 90.0;
const RADIUS: f64 = 0.2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub noclip: bool,
    pub speed: f64,
}

impl Settings {
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let speed = get("speed")
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .map_or(DEFAULT_SPEED, |v| v.clamp(MIN_SPEED, MAX_SPEED));
        Self {
            noclip: get("noclip").is_none_or(|v| v.trim() != "false"),
            speed,
        }
    }

    pub fn labels(&self) -> Vec<String> {
        vec![
            format!("Through walls: {}", if self.noclip { "ON" } else { "OFF" }),
            format!("Speed: {:.2} blocks per tick", self.speed),
        ]
    }

    pub fn click(&mut self, button: usize) {
        match button {
            0 => self.noclip = !self.noclip,
            1 => {
                self.speed = SPEEDS
                    .iter()
                    .copied()
                    .find(|s| *s > self.speed + 1e-9)
                    .unwrap_or(SPEEDS[0])
            }
            _ => {}
        }
    }

    pub fn stored(&self) -> [(&'static str, String); 2] {
        [
            ("noclip", self.noclip.to_string()),
            ("speed", self.speed.to_string()),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub at: [f64; 3],
    pub previous: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub view: u8,
}

#[derive(Default)]
struct State {
    settings: Option<Settings>,
    camera: Option<Camera>,
    held: [bool; 7],
    suspended: bool,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

pub fn settings(get: impl Fn(&str) -> Option<String>) -> Settings {
    STATE.with_borrow_mut(|s| *s.settings.get_or_insert_with(|| Settings::read(get)))
}

pub fn set_settings(settings: Settings) {
    STATE.with_borrow_mut(|s| s.settings = Some(settings));
}

pub fn active() -> bool {
    STATE.with_borrow(|s| s.camera.is_some())
}

pub fn start(eye: [f64; 3], yaw: f32, pitch: f32, view: u8) {
    STATE.with_borrow_mut(|s| {
        s.camera = Some(Camera {
            at: eye,
            previous: eye,
            yaw,
            pitch,
            view,
        });
        s.held = [false; 7];
        s.suspended = false;
    });
}

pub fn stop() -> Option<u8> {
    STATE.with_borrow_mut(|s| {
        s.held = [false; 7];
        s.suspended = false;
        s.camera.take().map(|c| c.view)
    })
}

/// Opening a menu or losing focus must not leave camera motion latched.
pub fn suspend(paused: bool) {
    STATE.with_borrow_mut(|s| {
        s.suspended = paused;
        if paused {
            s.held = [false; 7];
            if let Some(camera) = s.camera.as_mut() {
                camera.previous = camera.at;
            }
        }
    });
}

pub fn hold(input: usize, down: bool) {
    STATE.with_borrow_mut(|s| {
        if s.suspended {
            return;
        }
        if let Some(held) = s.held.get_mut(input) {
            *held = down;
        }
    });
}

pub fn turn(xo: f64, yo: f64) -> bool {
    STATE.with_borrow_mut(|s| {
        if s.suspended {
            return false;
        }
        let Some(c) = s.camera.as_mut() else {
            return false;
        };
        c.yaw += (xo * TURN) as f32;
        c.pitch = (c.pitch + (yo * TURN) as f32).clamp(-MAX_PITCH, MAX_PITCH);
        true
    })
}

pub fn tick(solid: impl Fn([i32; 3]) -> bool) {
    STATE.with_borrow_mut(|s| {
        if s.suspended {
            return;
        }
        let speed = s.settings.map_or(DEFAULT_SPEED, |c| c.speed);
        let held = s.held;
        let Some(c) = s.camera.as_mut() else { return };
        c.previous = c.at;
        c.at = slide(c.at, motion(c.yaw, held, speed), &solid);
    });
}

pub fn view(partial: f32) -> Option<([f64; 3], f32, f32)> {
    STATE.with_borrow(|s| {
        let c = s.camera?;
        let t = f64::from(partial);
        Some((
            std::array::from_fn(|i| c.previous[i] + (c.at[i] - c.previous[i]) * t),
            c.yaw,
            c.pitch,
        ))
    })
}

pub fn wheel(steps: f32) -> Option<Settings> {
    STATE.with_borrow_mut(|s| {
        s.camera?;
        let settings = s.settings.get_or_insert(Settings {
            noclip: true,
            speed: DEFAULT_SPEED,
        });
        settings.speed =
            (settings.speed * WHEEL_STEP.powf(f64::from(steps))).clamp(MIN_SPEED, MAX_SPEED);
        Some(*settings)
    })
}

pub fn motion(yaw: f32, held: [bool; 7], speed: f64) -> [f64; 3] {
    let axis = |plus: usize, minus: usize| {
        f64::from(u8::from(held[plus])) - f64::from(u8::from(held[minus]))
    };
    let (forward, strafe, rise) = (axis(FORWARD, BACK), axis(LEFT, RIGHT), axis(UP, DOWN));
    let speed = if held[SPRINT] {
        speed * SPRINT_FACTOR
    } else {
        speed
    };
    let yaw = f64::from(yaw).to_radians();
    let (x, z) = (
        strafe * yaw.cos() - forward * yaw.sin(),
        forward * yaw.cos() + strafe * yaw.sin(),
    );
    let length = x.hypot(z);
    let scale = if length > 0.0 { speed / length } else { 0.0 };
    [x * scale, rise * speed, z * scale]
}

pub fn slide(at: [f64; 3], delta: [f64; 3], solid: &impl Fn([i32; 3]) -> bool) -> [f64; 3] {
    if at.iter().chain(delta.iter()).any(|v| !v.is_finite()) {
        return at;
    }
    // Check along the path, not only the endpoint: Ctrl + maximum speed can
    // otherwise skip an entire wall. Tiny steps also preserve wall sliding.
    let steps = (delta.iter().fold(0.0_f64, |m, v| m.max(v.abs())) / 0.1)
        .ceil()
        .clamp(1.0, 4096.0) as u32;
    let step = delta.map(|v| v / f64::from(steps));
    let mut next = at;
    for _ in 0..steps {
        for axis in 0..3 {
            let mut moved = next;
            moved[axis] += step[axis];
            if !blocked(moved, solid) {
                next = moved;
            }
        }
    }
    next
}

fn blocked(at: [f64; 3], solid: &impl Fn([i32; 3]) -> bool) -> bool {
    let low = at.map(|v| (v - RADIUS).floor() as i32);
    let high = at.map(|v| (v + RADIUS).floor() as i32);
    (low[0]..=high[0])
        .any(|x| (low[1]..=high[1]).any(|y| (low[2]..=high[2]).any(|z| solid([x, y, z]))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |key| pairs.iter().find(|p| p.0 == key).map(|p| p.1.to_owned())
    }

    fn held(inputs: &[usize]) -> [bool; 7] {
        std::array::from_fn(|i| inputs.contains(&i))
    }

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
    }

    #[test]
    fn settings_default_round_trip_and_cycle() {
        assert_eq!(
            Settings::read(stored(&[])),
            Settings {
                noclip: true,
                speed: 1.0
            }
        );
        let s = Settings::read(stored(&[("noclip", "false"), ("speed", "99")]));
        assert_eq!(
            s,
            Settings {
                noclip: false,
                speed: MAX_SPEED
            }
        );
        assert_eq!(Settings::read(stored(&[("speed", "NaN")])).speed, 1.0);
        let back: Vec<(&str, String)> = s.stored().into();
        let pairs: Vec<(&str, &str)> = back.iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert_eq!(Settings::read(stored(&pairs)), s);
        let mut c = Settings {
            noclip: true,
            speed: 1.0,
        };
        c.click(1);
        assert_eq!(c.speed, 1.5);
        c.speed = 4.0;
        c.click(1);
        assert_eq!(c.speed, 0.25);
        c.click(0);
        assert_eq!(
            c.labels(),
            ["Through walls: OFF", "Speed: 0.25 blocks per tick"]
        );
    }

    #[test]
    fn moves_like_the_player_faces() {
        assert!(close(motion(0.0, held(&[FORWARD]), 1.0), [0.0, 0.0, 1.0]));
        assert!(close(motion(90.0, held(&[FORWARD]), 1.0), [-1.0, 0.0, 0.0]));
        assert!(close(motion(0.0, held(&[LEFT]), 1.0), [1.0, 0.0, 0.0]));
        assert!(close(
            motion(0.0, held(&[UP, SPRINT]), 0.5),
            [0.0, 1.0, 0.0]
        ));
        let diagonal = motion(0.0, held(&[FORWARD, LEFT]), 1.0);
        assert!((diagonal[0].hypot(diagonal[2]) - 1.0).abs() < 1e-9);
        assert!(close(motion(0.0, held(&[FORWARD, BACK]), 1.0), [0.0; 3]));
    }

    #[test]
    fn slides_along_walls_and_through_air() {
        let wall = |p: [i32; 3]| p[0] == 1;
        let next = slide([0.5, 0.5, 0.5], [0.6, 0.0, 0.4], &wall);
        assert!(next[0] < 0.8 && (next[2] - 0.9).abs() < 1e-9);
        assert!(close(
            slide([0.5, 0.5, 0.5], [0.6, 0.0, 0.4], &|_| false),
            [1.1, 0.5, 0.9]
        ));
    }

    #[test]
    fn fast_camera_cannot_skip_through_a_one_block_wall() {
        let next = slide([0.5, 1.5, 0.5], [8.0, 0.0, 0.0], &|p| p[0] == 1);
        assert!(next[0] <= 0.8, "camera tunneled through wall: {next:?}");
    }

    #[test]
    fn exiting_clears_motion_and_restores_the_saved_perspective_once() {
        start([0.0; 3], 12.0, 34.0, 2);
        hold(FORWARD, true);
        assert_eq!(stop(), Some(2));
        assert_eq!(stop(), None);
        STATE.with_borrow(|s| assert_eq!(s.held, [false; 7]));
        start([4.0, 5.0, 6.0], 12.0, 34.0, 0);
        tick(|_| false);
        assert_eq!(view(1.0).unwrap(), ([4.0, 5.0, 6.0], 12.0, 34.0));
        stop();
    }

    #[test]
    fn menus_and_focus_loss_clear_camera_motion_without_touching_saved_view() {
        start([2.0, 3.0, 4.0], 10.0, 20.0, 1);
        hold(FORWARD, true);
        suspend(true);
        hold(UP, true);
        assert!(!turn(100.0, 100.0));
        tick(|_| false);
        suspend(false);
        tick(|_| false);
        assert_eq!(view(1.0), Some(([2.0, 3.0, 4.0], 10.0, 20.0)));
        assert_eq!(stop(), Some(1));
    }

    #[test]
    fn camera_lifecycle_turns_without_the_player_and_interpolates() {
        assert!(!turn(10.0, 10.0));
        assert_eq!(view(0.5), None);
        start([0.0, 64.0, 0.0], 30.0, 10.0, 2);
        assert!(active());
        assert!(turn(100.0, 1000.0));
        let (_, yaw, pitch) = view(0.0).unwrap();
        assert_eq!((yaw, pitch), (45.0, 90.0));
        set_settings(Settings {
            noclip: true,
            speed: 1.0,
        });
        hold(UP, true);
        tick(|_| false);
        assert!(close(view(0.5).unwrap().0, [0.0, 64.5, 0.0]));
        assert!(close(view(1.0).unwrap().0, [0.0, 65.0, 0.0]));
        assert_eq!(wheel(1.0).map(|s| s.speed), Some(1.2));
        assert_eq!(wheel(100.0).map(|s| s.speed), Some(MAX_SPEED));
        assert_eq!(stop(), Some(2));
        assert!(!active());
        assert_eq!(wheel(1.0), None);
    }
}
