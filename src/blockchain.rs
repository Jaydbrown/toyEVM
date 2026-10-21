use std::sync::Arc;
use tokio::sync::RwLock;

use crate::rpc::mining::mining::BlockHeader;
use crate::rpc::state::encode;
use crate::transaction::EthTx;

#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<EthTx>,
}

impl Block {
    pub fn genesis() -> Self {
        let header = BlockHeader {
            parent_hash: [0u8; 32],
            state_root: [0u8; 32],
            transactions_root: [0u8; 32],
            number: 0,
            gas_limit: 30_000_000,
            gas_used: 0,
            timestamp: 1438269988, // Ethereum Frontier genesis timestamp
            difficulty: 1,
            nonce: 42,
        };
        Self {
            header,
            transactions: Vec::new(),
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        self.header.hash()
    }

    pub fn hash_hex(&self) -> String {
        encode(&self.hash())
    }
}

#[derive(Debug, Clone)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
}

impl Blockchain {
    pub fn new() -> Self {
        Self {
            blocks: vec![Block::genesis()],
        }
    }

    pub fn height(&self) -> u64 {
        self.blocks.len() as u64 - 1
    }

    pub fn best_block_hash(&self) -> [u8; 32] {
        self.blocks.last().unwrap().hash()
    }

    pub fn best_block_hash_hex(&self) -> String {
        encode(&self.best_block_hash())
    }

    pub fn add_block(&mut self, block: Block) -> Result<u64, &'static str> {
        // Rule: Parent block hash must match current canonical tip
        if block.header.parent_hash != self.best_block_hash() {
            return Err("Block rejected: parent_hash does not match chain tip");
        }

        // Rule: Header must satisfy Keccak-256 proof of work
        if !block.header.meets_difficulty(1) {
            return Err("Block rejected: invalid proof of work");
        }

        self.blocks.push(block);
        Ok(self.height())
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedBlockchain = Arc<RwLock<Blockchain>>;
