use std::collections::BTreeMap;
use std::fmt::Write;
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use serde::{Deserialize, Serialize};

use crate::{format_word_compact, Storage, ToyEvm};

pub fn decode(s: &str) -> Result<Vec<u8>, &'static str> {
    let trimmed = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    if trimmed.len() % 2 != 0 {
        return Err("Hex string must have an even number of characters");
    }

    let mut result = Vec::with_capacity(trimmed.len() / 2);

    for chunk in trimmed.as_bytes().chunks_exact(2) {
        let val1 = hex_val(chunk[0])?;
        let val2 = hex_val(chunk[1])?;
        result.push((val1 * 16) + val2);
    }

    Ok(result)
}

pub fn encode(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for &byte in b {
        let _ = write!(s, "{:02x}", byte);
    }
    s
}

fn hex_val(byte: u8) -> Result<u8, &'static str> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("invalid hex character"),
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct eval_evm_action {
    pub bytecode_hex: String,
    pub calldata_hex: Option<String>,
    pub value: Option<u128>,
    pub gas_limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalEvmResult {
    pub success: bool,
    pub reverted: bool,
    pub gas_used: u64,
    pub return_data: String,
    pub final_stack: Vec<String>,
    pub storage: BTreeMap<String, String>,
    pub error: Option<String>,
}

pub fn evaluate_evm(action: eval_evm_action) -> Result<EvalEvmResult, &'static str> {
    let bytecode = decode(&action.bytecode_hex)?;
    let calldata = match &action.calldata_hex {
        Some(hex) => decode(hex)?,
        None => Vec::new(),
    };
    let callvalue = action.value.unwrap_or(0);
    let gas_limit = action.gas_limit.unwrap_or(1_000_000);

    let evm = ToyEvm::with_context(bytecode, calldata, callvalue, gas_limit);

    match evm.execute(Storage::new()) {
        Ok(outcome) => {
            let final_stack: Vec<String> = outcome
                .stack
                .stack
                .iter()
                .map(|word| encode(&format_word_compact(word)))
                .collect();

            let mut storage_hex = BTreeMap::new();
            for (k, v) in &outcome.storage {
                storage_hex.insert(encode(k), encode(v));
            }

            Ok(EvalEvmResult {
                success: !outcome.reverted,
                reverted: outcome.reverted,
                gas_used: outcome.gas_used,
                return_data: encode(&outcome.return_data),
                final_stack,
                storage: storage_hex,
                error: if outcome.reverted {
                    Some("Execution reverted".to_string())
                } else {
                    None
                },
            })
        }
        Err(err_msg) => Ok(EvalEvmResult {
            success: false,
            reverted: false,
            gas_used: 0,
            return_data: String::new(),
            final_stack: Vec::new(),
            storage: BTreeMap::new(),
            error: Some(err_msg.to_string()),
        }),
    }
}

#[rpc(server)]
pub trait StateRpc {
    #[method(name = "eth_call")]
    async fn eth_call(&self, action: eval_evm_action) -> RpcResult<EvalEvmResult>;

    #[method(name = "eval_bytecode")]
    async fn eval_bytecode(&self, action: eval_evm_action) -> RpcResult<EvalEvmResult>;
}

pub struct StateRpcServerImpl;

#[jsonrpsee::core::async_trait]
impl StateRpcServer for StateRpcServerImpl {
    async fn eth_call(&self, action: eval_evm_action) -> RpcResult<EvalEvmResult> {
        evaluate_evm(action).map_err(|err| {
            jsonrpsee::types::ErrorObject::owned(-32602, err.to_string(), None::<()>)
        })
    }

    async fn eval_bytecode(&self, action: eval_evm_action) -> RpcResult<EvalEvmResult> {
        evaluate_evm(action).map_err(|err| {
            jsonrpsee::types::ErrorObject::owned(-32602, err.to_string(), None::<()>)
        })
    }
}
