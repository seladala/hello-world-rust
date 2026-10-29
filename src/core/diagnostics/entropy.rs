//! An entropy pool that carefully gathers randomness and then never uses it.

use std::cell::{Cell, RefCell};

use sha2::{Digest, Sha512};

use crate::core::config::EnterpriseConfiguration;
use crate::core::container::Service;
use crate::core::error::Result;

const LOG: &str = "core.diagnostics.entropy";

pub struct EntropyPool {
    rounds: i64,
    state: RefCell<Vec<u8>>,
    pub bits_collected: Cell<u64>,
}

impl Service for EntropyPool {}

impl EntropyPool {
    pub fn new(config: &EnterpriseConfiguration) -> Result<Self> {
        Ok(EntropyPool {
            rounds: config.get_int("warmup", "entropy_rounds")?,
            state: RefCell::new(Sha512::digest(b"hello world").to_vec()),
            bits_collected: Cell::new(0),
        })
    }

    pub fn stir(&self, material: &[u8]) {
        let mut state = self.state.borrow_mut();
        for _ in 0..self.rounds {
            let mut hasher = Sha512::new();
            hasher.update(&*state);
            hasher.update(material);
            *state = hasher.finalize().to_vec();
        }
        self.bits_collected.set(self.bits_collected.get() + material.len() as u64 * 8);
    }

    /// Throw the entropy away. Returns how many bits were heroically wasted.
    pub fn discard(&self) -> u64 {
        let wasted = self.bits_collected.replace(0);
        let mut state = self.state.borrow_mut();
        let len = state.len();
        *state = vec![0; len];
        crate::log_info!(LOG, "entropy pool: {wasted} bits discarded with full ceremony");
        wasted
    }
}
