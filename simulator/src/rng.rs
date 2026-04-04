/// Xoshiro256++ PRNG — fast, high-quality, reproducible from a seed.
#[derive(Debug, Clone, Default)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    /// Create a new RNG seeded from a single u64.
    /// Uses SplitMix64 to expand the seed into the full state.
    pub fn from_seed(seed: u64) -> Self {
        let mut sm = seed;
        let mut s = [0u64; 4];
        for slot in &mut s {
            *slot = splitmix64(&mut sm);
        }
        Rng { s }
    }

    /// Generate the next u64.
    pub fn next_u64(&mut self) -> u64 {
        let result = (self.s[0].wrapping_add(self.s[3]))
            .rotate_left(23)
            .wrapping_add(self.s[0]);

        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);

        result
    }

    /// Generate a uniform random number in [0, n).
    pub fn gen_range(&mut self, n: u64) -> u64 {
        if n <= 1 {
            return 0;
        }
        // Debiased modulo (Lemire's method simplified)
        loop {
            let x = self.next_u64();
            let r = x % n;
            if x - r <= u64::MAX - (n - 1) {
                return r;
            }
        }
    }

    /// Roll a die (1..=sides).
    pub fn roll_die(&mut self, sides: u8) -> u8 {
        (self.gen_range(sides as u64) + 1) as u8
    }

    /// Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.gen_range((i + 1) as u64) as usize;
            items.swap(i, j);
        }
    }

    /// Derive a child seed (for per-node RNGs).
    pub fn derive_seed(&mut self) -> u64 {
        self.next_u64()
    }
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a = Rng::from_seed(42);
        let mut b = Rng::from_seed(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = Rng::from_seed(42);
        let mut b = Rng::from_seed(43);
        // Extremely unlikely to collide
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn gen_range_in_bounds() {
        let mut rng = Rng::from_seed(123);
        for _ in 0..1000 {
            let v = rng.gen_range(6);
            assert!(v < 6);
        }
    }

    #[test]
    fn roll_die_in_bounds() {
        let mut rng = Rng::from_seed(456);
        for _ in 0..1000 {
            let v = rng.roll_die(6);
            assert!(v >= 1 && v <= 6);
        }
    }

    #[test]
    fn shuffle_preserves_elements() {
        let mut rng = Rng::from_seed(789);
        let mut items = vec![1, 2, 3, 4, 5];
        rng.shuffle(&mut items);
        items.sort();
        assert_eq!(items, vec![1, 2, 3, 4, 5]);
    }
}
