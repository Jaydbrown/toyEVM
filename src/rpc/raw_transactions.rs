use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use serde::{Deserialize, Serialize};

use super::state::{decode, encode};
use crate::transaction::EthTx;

pub fn decode_raw_transaction(hex: &str) -> Result<EthTx, &'static str> {
    let bytes = decode(hex)?;
    let mut cursor: usize = 0;

    if cursor + 8 > bytes.len() {
        return Err("Unexpected end of data reading chain_id");
    }
    let chain_id = u64::from_be_bytes(bytes[cursor..cursor + 8].try_into().unwrap());
    cursor += 8;

    if cursor + 8 > bytes.len() {
        return Err("Unexpected end of data reading nonce");
    }
    let nonce = u64::from_be_bytes(bytes[cursor..cursor + 8].try_into().unwrap());
    cursor += 8;

    if cursor + 8 > bytes.len() {
        return Err("Unexpected end of data reading gas_price");
    }
    let gas_price = u64::from_be_bytes(bytes[cursor..cursor + 8].try_into().unwrap());
    cursor += 8;

    if cursor + 8 > bytes.len() {
        return Err("Unexpected end of data reading gas_limit");
    }
    let gas_limit = u64::from_be_bytes(bytes[cursor..cursor + 8].try_into().unwrap());
    cursor += 8;

    if cursor >= bytes.len() {
        return Err("Unexpected end of data reading recipient flag");
    }
    let has_to = bytes[cursor];
    cursor += 1;

    let to = if has_to == 1 {
        if cursor + 20 > bytes.len() {
            return Err("Unexpected end of data reading 20-byte recipient address");
        }
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&bytes[cursor..cursor + 20]);
        cursor += 20;
        Some(addr)
    } else {
        None
    };

    if cursor + 16 > bytes.len() {
        return Err("Unexpected end of data reading value (wei)");
    }
    let value = u128::from_be_bytes(bytes[cursor..cursor + 16].try_into().unwrap());
    cursor += 16;

    if cursor + 4 > bytes.len() {
        return Err("Unexpected end of data reading calldata length");
    }
    let data_len = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
    cursor += 4;

    if cursor + data_len > bytes.len() {
        return Err("Unexpected end of data reading calldata bytes");
    }
    let data = bytes[cursor..cursor + data_len].to_vec();

    Ok(EthTx {
        chain_id,
        nonce,
        gas_price,
        gas_limit,
        to,
        value,
        data,
    })
}

pub fn encode_raw_transaction(tx: &EthTx) -> String {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&tx.chain_id.to_be_bytes());
    bytes.extend_from_slice(&tx.nonce.to_be_bytes());
    bytes.extend_from_slice(&tx.gas_price.to_be_bytes());
    bytes.extend_from_slice(&tx.gas_limit.to_be_bytes());

    match &tx.to {
        Some(addr) => {
            bytes.push(1);
            bytes.extend_from_slice(addr);
        }
        None => {
            bytes.push(0);
        }
    }

    bytes.extend_from_slice(&tx.value.to_be_bytes());
    bytes.extend_from_slice(&(tx.data.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&tx.data);

    encode(&bytes)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateRawTxParams {
    pub chain_id: Option<u64>,
    pub nonce: u64,
    pub gas_price: Option<u64>,
    pub gas_limit: Option<u64>,
    /// 40-char hex Ethereum address (omit or null for contract creation)
    pub to_hex: Option<String>,
    pub value: u128,
    pub data_hex: Option<String>,
}

pub fn create_raw_transaction(params: CreateRawTxParams) -> Result<String, &'static str> {
    let to = match &params.to_hex {
        Some(hex) if !hex.is_empty() => {
            let addr_bytes = decode(hex)?;
            if addr_bytes.len() != 20 {
                return Err("Recipient address (to_hex) must be exactly 20 bytes (40 hex chars)");
            }
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&addr_bytes);
            Some(addr)
        }
        _ => None,
    };

    let data = match &params.data_hex {
        Some(hex) => decode(hex)?,
        None => Vec::new(),
    };

    let tx = EthTx {
        chain_id: params.chain_id.unwrap_or(1337),
        nonce: params.nonce,
        gas_price: params.gas_price.unwrap_or(1_000_000_000),
        gas_limit: params.gas_limit.unwrap_or(21_000),
        to,
        value: params.value,
        data,
    };

    Ok(encode_raw_transaction(&tx))
}

#[rpc(server)]
pub trait RawTransactionsRpc {
    #[method(name = "decoderawtransaction")]
    async fn decode_raw_transaction(&self, hex: String) -> RpcResult<EthTx>;

    #[method(name = "createrawtransaction")]
    async fn create_raw_transaction(&self, params: CreateRawTxParams) -> RpcResult<String>;
}

pub struct RawTransactionsRpcServerImpl;

#[jsonrpsee::core::async_trait]
impl RawTransactionsRpcServer for RawTransactionsRpcServerImpl {
    async fn decode_raw_transaction(&self, hex: String) -> RpcResult<EthTx> {
        decode_raw_transaction(&hex).map_err(|err| {
            jsonrpsee::types::ErrorObject::owned(-32602, err.to_string(), None::<()>)
        })
    }

    async fn create_raw_transaction(&self, params: CreateRawTxParams) -> RpcResult<String> {
        create_raw_transaction(params).map_err(|err| {
            jsonrpsee::types::ErrorObject::owned(-32602, err.to_string(), None::<()>)
        })
    }
}