use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EthTx {
    pub chain_id: u64,
    pub nonce: u64,
    pub gas_price: u64,
    pub gas_limit: u64,
    /// 20-byte recipient address (`None` indicates contract creation)
    pub to: Option<[u8; 20]>,
    /// Transfer value in wei
    pub value: u128,
    /// Calldata or contract initialization bytecode
    pub data: Vec<u8>,
}

impl EthTx {
    pub fn new(
        chain_id: u64,
        nonce: u64,
        gas_price: u64,
        gas_limit: u64,
        to: Option<[u8; 20]>,
        value: u128,
        data: Vec<u8>,
    ) -> Self {
        Self {
            chain_id,
            nonce,
            gas_price,
            gas_limit,
            to,
            value,
            data,
        }
    }
}
