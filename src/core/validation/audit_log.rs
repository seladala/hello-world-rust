//! GlyphChain: a proof-of-work blockchain that records every approved character.
//!
//! Decentralised across exactly one process.

use std::cell::RefCell;
use std::rc::Rc;

use sha2::{Digest, Sha256};

use crate::core::config::EnterpriseConfiguration;
use crate::core::container::Service;
use crate::core::error::{self, Result};
use crate::core::events::EventBus;
use crate::core::json;
use crate::core::values::Value;

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub index: u64,
    pub data: Value,
    pub previous_hash: String,
    pub nonce: u64,
    pub hash: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn digest(index: u64, data: &Value, previous_hash: &str, nonce: u64) -> String {
    let body =
        Value::List(vec![Value::Int(index as i64), data.clone(), Value::str(previous_hash), Value::Int(nonce as i64)]);
    hex(&Sha256::digest(json::dumps(&body, true, &json::DEFAULT_SEPARATORS).as_bytes()))
}

pub struct GlyphChain {
    difficulty: usize,
    bus: Rc<EventBus>,
    pub blocks: RefCell<Vec<Block>>,
}

impl Service for GlyphChain {}

impl GlyphChain {
    pub fn new(config: &EnterpriseConfiguration, event_bus: Rc<EventBus>) -> Result<Self> {
        Ok(GlyphChain {
            difficulty: config.get_int("ledger", "difficulty")?.max(0) as usize,
            bus: event_bus,
            blocks: RefCell::new(Vec::new()),
        })
    }

    pub fn append(&self, data: Value) -> Result<Block> {
        let (index, previous) = {
            let blocks = self.blocks.borrow();
            (blocks.len() as u64, blocks.last().map_or_else(|| GENESIS_HASH.to_owned(), |b| b.hash.clone()))
        };
        let prefix = "0".repeat(self.difficulty);
        let mut nonce = 0;
        let hash = loop {
            let candidate = digest(index, &data, &previous, nonce);
            if candidate.starts_with(&prefix) {
                break candidate;
            }
            nonce += 1;
        };
        let block = Block { index, data, previous_hash: previous, nonce, hash: hash.clone() };
        self.blocks.borrow_mut().push(block.clone());
        self.bus.publish(
            "ledger.block.mined",
            Value::dict([
                ("index", Value::Int(index as i64)),
                ("nonce", Value::Int(nonce as i64)),
                ("hash", Value::str(&hash[..12])),
            ]),
        )?;
        Ok(block)
    }

    pub fn verify(&self) -> Result<()> {
        let mut previous = GENESIS_HASH.to_owned();
        for block in self.blocks.borrow().iter() {
            if block.previous_hash != previous {
                return Err(error::ledger_corruption(format!("block {} is not linked to its parent", block.index)));
            }
            if digest(block.index, &block.data, &block.previous_hash, block.nonce) != block.hash {
                return Err(error::ledger_corruption(format!("block {} has been tampered with", block.index)));
            }
            previous = block.hash.clone();
        }
        Ok(())
    }
}
