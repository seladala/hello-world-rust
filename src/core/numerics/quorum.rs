//! Byzantine-fault-tolerant consensus on the value of pi.
//!
//! Three independent algorithms each propose digits. A digit string is accepted
//! only when a quorum of oracles agrees on it. If the universe ever disagrees
//! with itself about pi, we will be the first to know.

use super::{high_precision, series, streaming};
use crate::core::error::{self, Result};

const LOG: &str = "core.numerics.quorum";

#[derive(Clone, Copy)]
pub struct PiOracle {
    pub name: &'static str,
    pub compute: fn(usize) -> String,
    pub max_digits: usize,
}

pub const ORACLES: [PiOracle; 3] = [
    PiOracle { name: "Chudnovsky (1988)", compute: high_precision::pi_digits, max_digits: 1_000_000 },
    PiOracle { name: "Machin (1706)", compute: series::pi_digits, max_digits: 50_000 },
    PiOracle { name: "Gibbons spigot (2006)", compute: streaming::pi_digits, max_digits: 2_000 },
];

pub struct PiConsensusProtocol {
    pub oracles: Vec<PiOracle>,
    pub quorum: usize,
}

impl PiConsensusProtocol {
    pub fn new(quorum: usize) -> Self {
        PiConsensusProtocol { oracles: ORACLES.to_vec(), quorum }
    }

    pub fn agree(&self, digits: usize) -> Result<String> {
        let mut proposals = Vec::new();
        for oracle in &self.oracles {
            let proposal = (oracle.compute)(digits.min(oracle.max_digits));
            crate::log_debug!(
                LOG,
                "oracle {:<22} proposes {}... ({} digits)",
                oracle.name,
                proposal.chars().take(12).collect::<String>(),
                proposal.chars().count()
            );
            proposals.push(proposal);
        }
        // Every full-length proposal is a candidate; oracles with a shorter
        // horizon endorse any candidate that extends what they can see.
        let mut candidates: Vec<&String> = Vec::new();
        for proposal in proposals.iter().filter(|p| p.chars().count() == digits) {
            if !candidates.contains(&proposal) {
                candidates.push(proposal);
            }
        }
        let mut best: Option<(&String, usize)> = None;
        for candidate in candidates {
            let votes = proposals.iter().filter(|p| candidate.starts_with(p.as_str())).count();
            if best.is_none_or(|(_, top)| votes > top) {
                best = Some((candidate, votes));
            }
        }
        let Some((winner, count)) = best else {
            return Err(error::pi_consensus_failure("no oracle could see far enough"));
        };
        if count < self.quorum {
            return Err(error::pi_consensus_failure(format!("no quorum on pi ({count}/{})", proposals.len())));
        }
        crate::log_info!(LOG, "pi consensus reached: {count}/{} oracles agree on {digits} digits", proposals.len());
        Ok(winner.clone())
    }
}
