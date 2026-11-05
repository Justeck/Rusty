// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// SDK-independent conservative corridor eligibility for native trajectory previews.

/// A conservative one-tick corridor includes native gravity, projectile bounds,
/// water queries, and neighbouring collision cells. Unknown columns end the
/// estimate before any simulation query can treat their missing data as air.
pub fn known_sweep(
    from: [f64; 3],
    velocity: [f64; 3],
    bottom: i32,
    top: i32,
    mut loaded: impl FnMut(i32, i32) -> bool,
) -> bool {
    if from.iter().chain(velocity.iter()).any(|v| !v.is_finite())
        || velocity.iter().any(|v| v.abs() > 32.0)
        || from.iter().any(|v| v.abs() > 30_000_000.0)
    {
        return false;
    }
    let to = std::array::from_fn::<_, 3, _>(|i| from[i] + velocity[i]);
    let lo = std::array::from_fn::<_, 3, _>(|i| (from[i].min(to[i]) - 1.0).floor() as i32);
    let hi = std::array::from_fn::<_, 3, _>(|i| (from[i].max(to[i]) + 1.0).floor() as i32);
    if lo[1] < bottom || hi[1] >= top {
        return false;
    }
    for x in (lo[0] >> 4)..=(hi[0] >> 4) {
        for z in (lo[2] >> 4)..=(hi[2] >> 4) {
            if !loaded(x, z) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::known_sweep;
    #[test]
    fn unknown_corridor_and_negative_chunk_boundaries_stop_prediction() {
        assert!(!known_sweep(
            [0.1, 64.0, 8.0],
            [-1.5, 0.0, 0.0],
            -64,
            320,
            |x, _| x >= 0
        ));
        assert!(!known_sweep(
            [15.5, 64.0, 8.0],
            [1.5, 0.0, 0.0],
            -64,
            320,
            |x, _| x == 0
        ));
        assert!(known_sweep(
            [8.0, 64.0, 8.0],
            [1.5, 0.0, 0.0],
            -64,
            320,
            |x, z| x == 0 && z == 0
        ));
    }
    #[test]
    fn invalid_motion_and_unknown_height_do_not_query_world() {
        for (at, v) in [
            ([f64::NAN, 64., 0.], [0.; 3]),
            ([0., 64., 0.], [f64::INFINITY, 0., 0.]),
            ([0., 64., 0.], [33., 0., 0.]),
            ([0., 319.5, 0.], [0.; 3]),
        ] {
            assert!(!known_sweep(at, v, -64, 320, |_, _| panic!(
                "invalid input queried world"
            )));
        }
    }
}
