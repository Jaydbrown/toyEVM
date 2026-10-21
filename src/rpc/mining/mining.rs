use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use serde::{Deserialize, Serialize};

use super::merkle::{compute_merkle_root, keccak256_hash};
use crate::rpc::state::{decode, encode};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub parent_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub transactions_root: [u8; 32],
    pub number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub difficulty: u32,
    pub nonce: u64,
}

impl BlockHeader {
    pub fn to_bytes(&self) -> [u8; 140] {
        let mut bytes = [0u8; 140];
        bytes[0..32].copy_from_slice(&self.parent_hash);
        bytes[32..64].copy_from_slice(&self.state_root);
        bytes[64..96].copy_from_slice(&self.transactions_root);
        bytes[96..104].copy_from_slice(&self.number.to_be_bytes());
        bytes[104..112].copy_from_slice(&self.gas_limit.to_be_bytes());
        bytes[112..120].copy_from_slice(&self.gas_used.to_be_bytes());
        bytes[120..128].copy_from_slice(&self.timestamp.to_be_bytes());
        bytes[128..132].copy_from_slice(&self.difficulty.to_be_bytes());
        bytes[132..140].copy_from_slice(&self.nonce.to_be_bytes());
        bytes
    }

    pub fn hash(&self) -> [u8; 32] {
        keccak256_hash(&self.to_bytes())
    }

    pub fn meets_difficulty(&self, leading_zero_bytes: usize) -> bool {
        let hash = self.hash();
        for &byte in &hash[..leading_zero_bytes] {
            if byte != 0 {
                return false;
            }
        }
        true
    }

    pub fn meets_difficulty_hex(&self, leading_zero_hex_chars: usize) -> bool {
        let hash_hex = encode(&self.hash());
        hash_hex.chars().take(leading_zero_hex_chars).all(|c| c == '0')
    }

    pub fn mine(&mut self, leading_zero_bytes: usize) -> (u64, [u8; 32]) {
        while !self.meets_difficulty(leading_zero_bytes) {
            self.nonce = self.nonce.wrapping_add(1);
        }
        (self.nonce, self.hash())
    }

    pub fn mine_hex(&mut self, leading_zero_hex_chars: usize) -> (u64, [u8; 32]) {
        while !self.meets_difficulty_hex(leading_zero_hex_chars) {
            self.nonce = self.nonce.wrapping_add(1);
        }
        (self.nonce, self.hash())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTemplate {
    pub parent_hash: String,
    pub number: u64,
    pub gas_limit: u64,
    pub difficulty: u32,
    pub transactions: Vec<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitBlockResult {
    pub accepted: bool,
    pub block_hash: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitBlockParams {
    pub header_hex: String,
    pub tx_hexes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateBlockResult {
    pub nonce: u64,
    pub block_hash: String,
    pub header_hex: String,
    pub number: u64,
}

pub fn generate_block(leading_zero_bytes: usize) -> GenerateBlockResult {
    let mut header = BlockHeader {
        parent_hash: [0u8; 32],
        state_root: [0u8; 32],
        transactions_root: [0u8; 32],
        number: 1,
        gas_limit: 30_000_000,
        gas_used: 0,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        difficulty: 1,
        nonce: 0,
    };

    let (nonce, hash) = header.mine(leading_zero_bytes);

    GenerateBlockResult {
        nonce,
        block_hash: encode(&hash),
        header_hex: encode(&header.to_bytes()),
        number: 1,
    }
}

pub fn verify_and_submit_block(params: SubmitBlockParams) -> Result<SubmitBlockResult, &'static str> {
    let header_bytes = decode(&params.header_hex)?;
    if header_bytes.len() != 140 {
        return Err("EVM block header must be exactly 140 bytes (280 hex characters)");
    }

    let mut parent_hash = [0u8; 32];
    parent_hash.copy_from_slice(&header_bytes[0..32]);

    let mut state_root = [0u8; 32];
    state_root.copy_from_slice(&header_bytes[32..64]);

    let mut transactions_root = [0u8; 32];
    transactions_root.copy_from_slice(&header_bytes[64..96]);

    let header = BlockHeader {
        parent_hash,
        state_root,
        transactions_root,
        number: u64::from_be_bytes(header_bytes[96..104].try_into().unwrap()),
        gas_limit: u64::from_be_bytes(header_bytes[104..112].try_into().unwrap()),
        gas_used: u64::from_be_bytes(header_bytes[112..120].try_into().unwrap()),
        timestamp: u64::from_be_bytes(header_bytes[120..128].try_into().unwrap()),
        difficulty: u32::from_be_bytes(header_bytes[128..132].try_into().unwrap()),
        nonce: u64::from_be_bytes(header_bytes[132..140].try_into().unwrap()),
    };

    let mut tx_hashes = Vec::new();
    for tx_hex in &params.tx_hexes {
        let raw_tx = decode(tx_hex)?;
        tx_hashes.push(keccak256_hash(&raw_tx));
    }

    if !tx_hashes.is_empty() {
        let computed_root = compute_merkle_root(tx_hashes)?;
        if computed_root != header.transactions_root {
            return Ok(SubmitBlockResult {
                accepted: false,
                block_hash: encode(&header.hash()),
                error: Some("Transactions root mismatch".to_string()),
            });
        }
    }

    let block_hash = encode(&header.hash());
    Ok(SubmitBlockResult {
        accepted: true,
        block_hash,
        error: None,
    })
}

#[rpc(server)]
pub trait MiningRpc {
    #[method(name = "getblocktemplate")]
    async fn get_block_template(&self) -> RpcResult<BlockTemplate>;

    #[method(name = "submitblock")]
    async fn submit_block(&self, params: SubmitBlockParams) -> RpcResult<SubmitBlockResult>;

    #[method(name = "generate")]
    async fn generate(&self, difficulty_zero_bytes: Option<usize>) -> RpcResult<GenerateBlockResult>;
}

pub struct MiningRpcServerImpl {
    pub chain: Option<crate::blockchain::SharedBlockchain>,
}

impl MiningRpcServerImpl {
    pub fn new(chain: crate::blockchain::SharedBlockchain) -> Self {
        Self { chain: Some(chain) }
    }
}

impl Default for MiningRpcServerImpl {
    fn default() -> Self {
        Self { chain: None }
    }
}

#[jsonrpsee::core::async_trait]
impl MiningRpcServer for MiningRpcServerImpl {
    async fn get_block_template(&self) -> RpcResult<BlockTemplate> {
        let (parent_hash, number) = if let Some(chain) = &self.chain {
            let c = chain.read().await;
            (c.best_block_hash_hex(), c.height() + 1)
        } else {
            (
                "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                1,
            )
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(BlockTemplate {
            parent_hash,
            number,
            gas_limit: 30_000_000,
            difficulty: 1,
            transactions: Vec::new(),
            timestamp,
        })
    }

    async fn submit_block(&self, params: SubmitBlockParams) -> RpcResult<SubmitBlockResult> {
        verify_and_submit_block(params).map_err(|err| {
            jsonrpsee::types::ErrorObject::owned(-32602, err.to_string(), None::<()>)
        })
    }

    async fn generate(
        &self,
        difficulty_zero_bytes: Option<usize>,
    ) -> RpcResult<GenerateBlockResult> {
        let diff = difficulty_zero_bytes.unwrap_or(1);

        let (parent_hash, number) = if let Some(chain) = &self.chain {
            let c = chain.read().await;
            (c.best_block_hash(), c.height() + 1)
        } else {
            ([0u8; 32], 1)
        };

        let mut header = BlockHeader {
            parent_hash,
            state_root: [0u8; 32],
            transactions_root: [0u8; 32],
            number,
            gas_limit: 30_000_000,
            gas_used: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            difficulty: 1,
            nonce: 0,
        };

        let (nonce, hash) = header.mine(diff);

        if let Some(chain) = &self.chain {
            let mut c = chain.write().await;
            let block = crate::blockchain::Block {
                header: header.clone(),
                transactions: Vec::new(),
            };
            let _ = c.add_block(block);
        }

        Ok(GenerateBlockResult {
            nonce,
            block_hash: encode(&hash),
            header_hex: encode(&header.to_bytes()),
            number,
        })
    }
}
