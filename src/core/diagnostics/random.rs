//! A Mersenne Twister (MT19937) seeded, sampled and shuffled exactly like the reference
//! implementation's `random.Random`, so that bogosort takes the same number of attempts.

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_B0DF;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7FFF_FFFF;

pub struct Random {
    state: [u32; N],
    index: usize,
}

impl Random {
    pub fn new(seed: u64) -> Random {
        let mut key: Vec<u32> = Vec::new();
        let mut rest = seed;
        while rest > 0 {
            key.push(rest as u32);
            rest >>= 32;
        }
        if key.is_empty() {
            key.push(0);
        }
        let mut rng = Random { state: [0; N], index: N + 1 };
        rng.init_by_array(&key);
        rng
    }

    fn init_genrand(&mut self, seed: u32) {
        self.state[0] = seed;
        for i in 1..N {
            let previous = self.state[i - 1];
            self.state[i] = 1_812_433_253u32.wrapping_mul(previous ^ (previous >> 30)).wrapping_add(i as u32);
        }
        self.index = N;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19_650_218);
        let (mut i, mut j) = (1usize, 0usize);
        for _ in 0..N.max(key.len()) {
            let previous = self.state[i - 1];
            self.state[i] = (self.state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                self.state[0] = self.state[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..N - 1 {
            let previous = self.state[i - 1];
            self.state[i] =
                (self.state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_566_083_941)).wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                self.state[0] = self.state[N - 1];
                i = 1;
            }
        }
        self.state[0] = 0x8000_0000;
    }

    fn twist(&mut self) {
        let mag01 = [0u32, MATRIX_A];
        for kk in 0..N {
            let y = (self.state[kk] & UPPER_MASK) | (self.state[(kk + 1) % N] & LOWER_MASK);
            self.state[kk] = self.state[(kk + M) % N] ^ (y >> 1) ^ mag01[(y & 1) as usize];
        }
        self.index = 0;
    }

    pub fn genrand_u32(&mut self) -> u32 {
        if self.index >= N {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9D2C_5680;
        y ^= (y << 15) & 0xEFC6_0000;
        y ^ (y >> 18)
    }

    /// Random integer with `k` random bits, for 0 < k <= 32.
    pub fn getrandbits(&mut self, k: u32) -> u32 {
        assert!((1..=32).contains(&k), "getrandbits supports 1..=32 bits here");
        self.genrand_u32() >> (32 - k)
    }

    /// Uniform integer in [0, n), by rejection sampling on the bit length of n.
    pub fn randbelow(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        let k = usize::BITS - n.leading_zeros();
        loop {
            let r = self.getrandbits(k) as usize;
            if r < n {
                return r;
            }
        }
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.randbelow(i + 1);
            items.swap(i, j);
        }
    }

    pub fn sample<T: Clone>(&mut self, population: &[T], k: usize) -> Vec<T> {
        let n = population.len();
        assert!(k <= n, "Sample larger than population or is negative");
        let mut setsize = 21usize;
        if k > 5 {
            setsize += 4usize.pow((3.0 * k as f64).log(4.0).ceil() as u32);
        }
        let mut result = Vec::with_capacity(k);
        if n <= setsize {
            let mut pool = population.to_vec();
            for i in 0..k {
                let j = self.randbelow(n - i);
                result.push(pool[j].clone());
                pool[j] = pool[n - i - 1].clone();
            }
        } else {
            let mut selected = std::collections::HashSet::new();
            for _ in 0..k {
                let mut j = self.randbelow(n);
                while selected.contains(&j) {
                    j = self.randbelow(n);
                }
                selected.insert(j);
                result.push(population[j].clone());
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_reference_sequence() {
        let mut rng = Random::new(0xC0FFEE);
        let population: Vec<u32> = (0..100).collect();
        assert_eq!(rng.sample(&population, 5), vec![74, 79, 22, 2, 15]);
    }
}
