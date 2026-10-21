use std::sync::Arc;
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use serde::{Deserialize, Serialize};

use crate::p2p::Swarm;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainInfo {
    pub chain: String,
    pub chain_id: u64,
    pub blocks: u64,
    pub best_block_hash: String,
    pub difficulty: u32,
    pub gas_limit: u64,
    pub syncing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub version: String,
    pub client_version: String,
    pub protocol_version: u32,
    pub connections: u32,
    pub network_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolInfo {
    pub pending: usize,
    pub bytes: usize,
    pub usage: usize,
}

#[rpc(server)]
pub trait NodeRpc {
    #[method(name = "getblockchaininfo")]
    async fn get_blockchain_info(&self) -> RpcResult<BlockchainInfo>;

    #[method(name = "getnetworkinfo")]
    async fn get_network_info(&self) -> RpcResult<NetworkInfo>;

    #[method(name = "getmempoolinfo")]
    async fn get_mempool_info(&self) -> RpcResult<MempoolInfo>;
}

pub struct NodeRpcServerImpl {
    pub swarm: Option<Arc<Swarm>>,
    pub chain: Option<crate::blockchain::SharedBlockchain>,
}

impl NodeRpcServerImpl {
    pub fn new(swarm: Arc<Swarm>, chain: crate::blockchain::SharedBlockchain) -> Self {
        Self {
            swarm: Some(swarm),
            chain: Some(chain),
        }
    }
}

impl Default for NodeRpcServerImpl {
    fn default() -> Self {
        Self {
            swarm: None,
            chain: None,
        }
    }
}

#[jsonrpsee::core::async_trait]
impl NodeRpcServer for NodeRpcServerImpl {
    async fn get_blockchain_info(&self) -> RpcResult<BlockchainInfo> {
        let (blocks, best_block_hash) = if let Some(chain) = &self.chain {
            let c = chain.read().await;
            (c.height(), c.best_block_hash_hex())
        } else {
            (
                0,
                "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            )
        };

        Ok(BlockchainInfo {
            chain: "toyEVM".to_string(),
            chain_id: 1337,
            blocks,
            best_block_hash,
            difficulty: 1,
            gas_limit: 30_000_000,
            syncing: false,
        })
    }

    async fn get_network_info(&self) -> RpcResult<NetworkInfo> {
        let connections = if let Some(swarm) = &self.swarm {
            swarm.peer_count().await as u32
        } else {
            0
        };

        Ok(NetworkInfo {
            version: "0.1.0".to_string(),
            client_version: "ToyEVM/v0.1.0/rust".to_string(),
            protocol_version: 68,
            connections,
            network_active: true,
        })
    }

    async fn get_mempool_info(&self) -> RpcResult<MempoolInfo> {
        let (pending, bytes, usage) = if let Some(swarm) = &self.swarm {
            swarm.mempool_info().await
        } else {
            (0, 0, 0)
        };

        Ok(MempoolInfo {
            pending,
            bytes,
            usage,
        })
    }
}
