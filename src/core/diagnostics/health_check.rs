//! Warm-up routines. None of them influence the output. All of them are mandatory.

use std::collections::HashMap;
use std::rc::Rc;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use super::entropy::EntropyPool;
use super::random::Random;
use crate::core::config::EnterpriseConfiguration;
use crate::core::container::Service;
use crate::core::error::{self, Result};

const LOG: &str = "core.diagnostics.health_check";

pub const KNOWN_PRIME_COUNTS: [(i64, usize); 4] = [(1000, 168), (10000, 1229), (20000, 2262), (100000, 9592)];

/// Check the Collatz conjecture up to `limit`. Returns the longest trajectory found.
pub fn verify_collatz(limit: u64) -> u64 {
    let mut known: HashMap<u64, u64> = HashMap::from([(1, 0)]);
    let mut longest = 0;
    for start in 1..=limit {
        let mut path = Vec::new();
        let mut n = start;
        while !known.contains_key(&n) {
            path.push(n);
            n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        }
        let mut steps = known[&n];
        for &value in path.iter().rev() {
            steps += 1;
            known.insert(value, steps);
        }
        longest = longest.max(known[&start]);
    }
    longest
}

pub fn count_primes(limit: usize) -> usize {
    let mut sieve = vec![1u8; limit + 1];
    for slot in sieve.iter_mut().take(2) {
        *slot = 0;
    }
    let bound = (limit as f64).sqrt() as usize + 1;
    for i in 2..bound {
        if sieve[i] == 1 {
            for multiple in (i * i..=limit).step_by(i) {
                sieve[multiple] = 0;
            }
        }
    }
    sieve.iter().map(|&b| b as usize).sum()
}

/// Gaussian elimination on the identity matrix, in exact rational arithmetic.
pub fn identity_determinant(size: usize) -> Result<BigRational> {
    let fraction = |v: i64| BigRational::from_integer(BigInt::from(v));
    let mut m: Vec<Vec<BigRational>> =
        (0..size).map(|r| (0..size).map(|c| fraction((r == c) as i64)).collect()).collect();
    let mut det = BigRational::one();
    for col in 0..size {
        let pivot = (col..size)
            .find(|&r| !m[r][col].is_zero())
            .ok_or_else(|| error::runtime_error("generator raised StopIteration"))?;
        if pivot != col {
            m.swap(col, pivot);
            det = -det;
        }
        det *= &m[col][col];
        for r in col + 1..size {
            let factor = &m[r][col] / &m[col][col];
            let reduced: Vec<BigRational> = m[r].iter().zip(&m[col]).map(|(a, b)| a - &factor * b).collect();
            m[r] = reduced;
        }
    }
    Ok(det)
}

pub fn bogosort(items: &[i64], rng: &mut Random) -> (Vec<i64>, u64) {
    let mut items = items.to_vec();
    let mut attempts = 0;
    while items.windows(2).any(|pair| pair[0] > pair[1]) {
        rng.shuffle(&mut items);
        attempts += 1;
    }
    (items, attempts)
}

fn repr_list(items: &[i64]) -> String {
    format!("[{}]", items.iter().map(i64::to_string).collect::<Vec<_>>().join(", "))
}

pub struct WarmupOrchestrator {
    config: Rc<EnterpriseConfiguration>,
    entropy: Rc<EntropyPool>,
}

impl WarmupOrchestrator {
    pub fn new(config: Rc<EnterpriseConfiguration>, entropy_pool: Rc<EntropyPool>) -> Self {
        WarmupOrchestrator { config, entropy: entropy_pool }
    }
}

impl Service for WarmupOrchestrator {
    fn on_start(&self) -> Result<()> {
        let cfg = &self.config;
        let longest = verify_collatz(cfg.get_int("warmup", "collatz_limit")?.max(0) as u64);
        crate::log_info!(LOG, "warmup: Collatz conjecture holds (longest trajectory: {longest} steps)");

        let limit = cfg.get_int("warmup", "sieve_limit")?;
        let primes = count_primes(limit.max(1) as usize);
        if let Some(&(_, expected)) = KNOWN_PRIME_COUNTS.iter().find(|(l, _)| *l == limit) {
            if primes != expected {
                return Err(error::warmup_failure(format!("number theory is broken: pi({limit}) = {primes}")));
            }
        }
        crate::log_info!(LOG, "warmup: {primes} primes below {limit}, as expected");

        let det = identity_determinant(cfg.get_int("warmup", "matrix_size")?.max(0) as usize)?;
        if !det.is_one() {
            return Err(error::warmup_failure(format!("det(I) = {det}")));
        }
        crate::log_info!(LOG, "warmup: det(I) = 1, linear algebra operational");

        let mut rng = Random::new(0xC0FFEE);
        let size = cfg.get_int("warmup", "bogosort_size")?.max(0) as usize;
        let population: Vec<i64> = (0..100).collect();
        let (result, attempts) = bogosort(&rng.sample(&population, size), &mut rng);
        crate::log_info!(LOG, "warmup: bogosort sorted {} in {attempts} attempts", repr_list(&result));

        let material = format!(
            "({longest}, {primes}, Fraction({}, {}), {}, {attempts})",
            det.numer(),
            det.denom(),
            repr_list(&result)
        );
        self.entropy.stir(material.as_bytes());
        self.entropy.discard();
        Ok(())
    }
}
