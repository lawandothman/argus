//! A small, fast, seedable pseudo-random generator (SplitMix64).
//!
//! Dependency-free and deterministic for a given seed — so simulated telemetry
//! is reproducible when seeded with `ARGUS_SEED`, and lively when seeded from
//! the clock.

/// A SplitMix64 pseudo-random generator.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Seed the generator.
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    /// The next raw 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A uniform float in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        // Use the top 53 bits for full f64 mantissa precision.
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// A uniform integer in `[low, high)`; returns `low` if the range is empty.
    pub fn range(&mut self, low: u64, high: u64) -> u64 {
        if high <= low {
            return low;
        }
        low + self.next_u64() % (high - low)
    }

    /// Returns `true` with probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    /// A random 16-byte array (for trace identifiers).
    pub fn bytes16(&mut self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[..8].copy_from_slice(&self.next_u64().to_le_bytes());
        out[8..].copy_from_slice(&self.next_u64().to_le_bytes());
        out
    }

    /// A random 8-byte array (for span identifiers).
    pub fn bytes8(&mut self) -> [u8; 8] {
        self.next_u64().to_le_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_a_given_seed() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        assert_eq!(a.next_u64(), b.next_u64());
        assert_eq!(a.bytes16(), b.bytes16());
    }

    #[test]
    fn floats_stay_in_unit_interval() {
        let mut rng = Rng::new(7);
        for _ in 0..1_000 {
            let value = rng.next_f64();
            assert!((0.0..1.0).contains(&value));
        }
    }

    #[test]
    fn range_is_bounded() {
        let mut rng = Rng::new(99);
        for _ in 0..1_000 {
            let value = rng.range(10, 20);
            assert!((10..20).contains(&value));
        }
    }
}
