//! Seeded randomness. One ChaCha8 generator per night, derived from the game
//! seed and the night number. Only 32-bit draws are used so results match on
//! every platform (no usize-dependent ranges).

use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};

pub struct Rng(ChaCha8Rng);

impl Rng {
    pub fn for_night(seed: u64, night: u8, salt: u64) -> Rng {
        let mixed = seed
            ^ (night as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ salt.wrapping_mul(0xD1B5_4A32_D192_ED03);
        Rng(ChaCha8Rng::seed_from_u64(mixed))
    }

    pub fn u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    /// Uniform in 0..n (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        if n <= 1 {
            return 0;
        }
        // Lemire-style multiply-shift; unbiased enough for a board game and fully deterministic.
        ((self.u32() as u64 * n as u64) >> 32) as u32
    }

    /// True with probability p per mille.
    pub fn chance(&mut self, permille: i32) -> bool {
        if permille <= 0 {
            return false;
        }
        if permille >= 1000 {
            return true;
        }
        (self.below(1000) as i32) < permille
    }

    /// Pick an index weighted by `weights` (all >= 0). None if all are zero.
    pub fn weighted(&mut self, weights: &[i32]) -> Option<usize> {
        let total: i64 = weights.iter().map(|&w| w.max(0) as i64).sum();
        if total <= 0 {
            return None;
        }
        let mut r = (self.u32() as i64 * total) >> 32;
        for (i, &w) in weights.iter().enumerate() {
            let w = w.max(0) as i64;
            if r < w {
                return Some(i);
            }
            r -= w;
        }
        Some(weights.len() - 1)
    }
}

/// A small deterministic hash for flavor choices that must not disturb the sim RNG.
pub fn mix(a: u64, b: u64) -> u64 {
    let mut z = a ^ b.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// FNV-1a over bytes; used for state hashes in saves and replays.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
