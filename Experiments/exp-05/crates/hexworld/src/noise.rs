//! Hashed-lattice gradient noise. Pure, seeded, and dependency-free: the same point
//! always gives the same value, whichever chunk or thread asks.

/// A 64-bit mix (splitmix64's finaliser) for hashing lattice points.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut x = z;
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn gradient(seed: u64, ix: i64, iy: i64) -> (f64, f64) {
    let h = mix(seed ^ mix((ix as u64).wrapping_mul(0x1234_5678_9ABC_DEF1) ^ (iy as u64)));
    let angle = (h >> 11) as f64 / (1u64 << 53) as f64 * std::f64::consts::TAU;
    (angle.cos(), angle.sin())
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Gradient noise at a point, in [-1, 1].
pub fn gradient_2d(seed: u64, x: f64, y: f64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let (ix, iy) = (x0 as i64, y0 as i64);
    let (fx, fy) = (x - x0, y - y0);
    let mut corners = [0.0; 4];
    for (i, slot) in corners.iter_mut().enumerate() {
        let (dx, dy) = ((i & 1) as f64, (i >> 1) as f64);
        let g = gradient(seed, ix + dx as i64, iy + dy as i64);
        *slot = g.0 * (fx - dx) + g.1 * (fy - dy);
    }
    let (u, v) = (fade(fx), fade(fy));
    let top = corners[0] + u * (corners[1] - corners[0]);
    let bottom = corners[2] + u * (corners[3] - corners[2]);
    // Gradient noise peaks near 1/sqrt(2); scale so the range is about [-1, 1].
    ((top + v * (bottom - top)) * std::f64::consts::SQRT_2).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_deterministic() {
        assert_eq!(gradient_2d(7, 1.5, -2.25), gradient_2d(7, 1.5, -2.25));
    }

    #[test]
    fn depends_on_the_seed() {
        assert_ne!(gradient_2d(1, 3.3, 4.4), gradient_2d(2, 3.3, 4.4));
    }

    #[test]
    fn stays_in_range_and_varies() {
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        for i in 0..2_000 {
            let x = i as f64 * 0.37;
            let y = i as f64 * -0.11;
            let v = gradient_2d(42, x, y);
            assert!((-1.0..=1.0).contains(&v), "{v} out of range");
            min = min.min(v);
            max = max.max(v);
        }
        assert!(max - min > 0.5, "noise barely varies: {min}..{max}");
    }

    #[test]
    fn is_zero_at_lattice_points() {
        // Gradient noise vanishes on the lattice; this catches a value-noise mix-up.
        for i in -3..=3 {
            assert!(gradient_2d(9, i as f64, 2.0).abs() < 1e-12);
        }
    }
}
