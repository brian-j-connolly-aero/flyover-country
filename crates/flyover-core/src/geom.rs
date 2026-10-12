//! Integer geometry, so native and WebAssembly builds agree to the last unit.

pub type P = (i32, i32);

pub fn dist2(a: P, b: P) -> i64 {
    let dx = (b.0 - a.0) as i64;
    let dy = (b.1 - a.1) as i64;
    dx * dx + dy * dy
}

/// Floor of the square root.
pub fn isqrt(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut x = (n as f64).sqrt() as i64; // a starting guess only; corrected below
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x
}

pub fn dist(a: P, b: P) -> i32 {
    isqrt(dist2(a, b)) as i32
}

/// The point `s` units along the segment from `a` to `b` (clamped to the segment).
pub fn along(a: P, b: P, s: i32) -> P {
    let len = dist(a, b);
    if len <= 0 || s >= len {
        return b;
    }
    if s <= 0 {
        return a;
    }
    let (s, len) = (s as i64, len as i64);
    (
        a.0 + ((b.0 - a.0) as i64 * s / len) as i32,
        a.1 + ((b.1 - a.1) as i64 * s / len) as i32,
    )
}

/// How far along the segment `a`→`b` it first enters the circle at `c` with
/// radius `r`, in map units. Zero if it starts inside; None if it never enters.
pub fn circle_entry(a: P, b: P, c: P, r: i32) -> Option<i32> {
    let r2 = (r as i64) * (r as i64);
    if dist2(a, c) <= r2 {
        return Some(0);
    }
    let (dx, dy) = ((b.0 - a.0) as i64, (b.1 - a.1) as i64);
    let (fx, fy) = ((a.0 - c.0) as i64, (a.1 - c.1) as i64);
    let dd = dx * dx + dy * dy;
    if dd == 0 {
        return None;
    }
    let fd = fx * dx + fy * dy;
    let ff = fx * fx + fy * fy;
    let disc = fd * fd - dd * (ff - r2);
    if disc < 0 {
        return None;
    }
    let num = -fd - isqrt(disc);
    if num < 0 || num > dd {
        return None;
    }
    let len = isqrt(dd);
    Some((num * len / dd) as i32)
}

/// Even-odd point-in-polygon test.
pub fn inside(p: P, poly: &[P]) -> bool {
    let (px, py) = (p.0 as i64, p.1 as i64);
    let mut c = false;
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (poly[i].0 as i64, poly[i].1 as i64);
        let (xj, yj) = (poly[j].0 as i64, poly[j].1 as i64);
        if (yi > py) != (yj > py) {
            // px < xi + (xj - xi) * (py - yi) / (yj - yi), without division
            let lhs = (px - xi) * (yj - yi);
            let rhs = (xj - xi) * (py - yi);
            if (yj - yi > 0 && lhs < rhs) || (yj - yi < 0 && lhs > rhs) {
                c = !c;
            }
        }
        j = i;
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt_is_exact_floor() {
        for n in 0..10_000i64 {
            let r = isqrt(n);
            assert!(r * r <= n && (r + 1) * (r + 1) > n);
        }
    }

    #[test]
    fn entry_points() {
        // A horizontal path through a circle centered on it enters r before the center.
        assert_eq!(circle_entry((0, 0), (200, 0), (100, 0), 30), Some(70));
        // Starting inside counts as entering at once.
        assert_eq!(circle_entry((95, 0), (200, 0), (100, 0), 30), Some(0));
        // Passing wide misses.
        assert_eq!(circle_entry((0, 0), (200, 0), (100, 50), 30), None);
        // Stopping short misses.
        assert_eq!(circle_entry((0, 0), (60, 0), (100, 0), 30), None);
    }

    #[test]
    fn polygon() {
        let sq = [(0, 0), (10, 0), (10, 10), (0, 10)];
        assert!(inside((5, 5), &sq));
        assert!(!inside((15, 5), &sq));
    }
}
