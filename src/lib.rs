#![allow(non_snake_case)]

pub mod blockchain;
pub mod executionStack;
pub mod opcode;
pub mod p2p;
pub mod rpc;
pub mod transaction;

pub use executionStack::{Memory, Stack, Storage};
pub use opcode::Opcode;

use sha3::{Digest, Keccak256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action<'a> {
    ExecuteOpcode(Opcode),
    PushData(&'a [u8]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionOutcome {
    pub stack: Stack,
    pub memory: Memory,
    pub storage: Storage,
    pub return_data: Vec<u8>,
    pub gas_used: u64,
    pub reverted: bool,
}

pub struct ToyEvm {
    bytecode: Vec<u8>,
    calldata: Vec<u8>,
    callvalue: u128,
    gas_limit: u64,
}

impl ToyEvm {
    pub fn new(bytecode: Vec<u8>) -> Self {
        Self {
            bytecode,
            calldata: Vec::new(),
            callvalue: 0,
            gas_limit: 1_000_000,
        }
    }

    pub fn with_context(
        bytecode: Vec<u8>,
        calldata: Vec<u8>,
        callvalue: u128,
        gas_limit: u64,
    ) -> Self {
        Self {
            bytecode,
            calldata,
            callvalue,
            gas_limit,
        }
    }

    pub fn read_action(&self, pc: usize) -> Result<(Action<'_>, usize, u64), &'static str> {
        if pc >= self.bytecode.len() {
            return Err("Program counter exceeded bytecode length");
        }

        let op = self.bytecode[pc];
        match op {
            0x5f => Ok((Action::PushData(&[]), 1, Opcode::Push0.gas_cost())),
            0x60..=0x7f => {
                let len = (op - 0x5f) as usize;
                let start = pc + 1;
                let end = start + len;

                if end > self.bytecode.len() {
                    return Err("PUSH immediate data exceeds bytecode bounds");
                }

                let data = &self.bytecode[start..end];
                Ok((Action::PushData(data), 1 + len, 3))
            }
            _ => {
                let opcode = Opcode::try_from(op)?;
                Ok((Action::ExecuteOpcode(opcode), 1, opcode.gas_cost()))
            }
        }
    }

    pub fn run(&self) -> Result<Stack, &'static str> {
        let outcome = self.execute(Storage::new())?;
        if outcome.reverted {
            return Err("EVM execution reverted");
        }
        Ok(outcome.stack)
    }

    pub fn execute(&self, mut storage: Storage) -> Result<ExecutionOutcome, &'static str> {
        let mut stack = Stack::new();
        let mut memory = Memory::new();
        let mut pc: usize = 0;
        let mut gas_used: u64 = 0;

        while pc < self.bytecode.len() {
            let current_pc = pc;
            let (action, consumed, step_gas) = self.read_action(pc)?;

            if gas_used.saturating_add(step_gas) > self.gas_limit {
                return Err("Out of gas");
            }
            gas_used += step_gas;

            match action {
                Action::PushData(data) => {
                    stack.push_bytes(data)?;
                    pc += consumed;
                }
                Action::ExecuteOpcode(op) => match op {
                    Opcode::Stop => {
                        return Ok(ExecutionOutcome {
                            stack,
                            memory,
                            storage,
                            return_data: Vec::new(),
                            gas_used,
                            reverted: false,
                        });
                    }
                    Opcode::Add => {
                        let a = stack.pop_u128()?;
                        let b = stack.pop_u128()?;
                        stack.push_u128(a.wrapping_add(b))?;
                        pc += consumed;
                    }
                    Opcode::Mul => {
                        let a = stack.pop_u128()?;
                        let b = stack.pop_u128()?;
                        stack.push_u128(a.wrapping_mul(b))?;
                        pc += consumed;
                    }
                    Opcode::Sub => {
                        let a = stack.pop_u128()?;
                        let b = stack.pop_u128()?;
                        stack.push_u128(a.wrapping_sub(b))?;
                        pc += consumed;
                    }
                    Opcode::Div => {
                        let a = stack.pop_u128()?;
                        let b = stack.pop_u128()?;
                        // EVM specification: division by zero yields 0, not a panic/revert
                        stack.push_u128(if b == 0 { 0 } else { a / b })?;
                        pc += consumed;
                    }
                    Opcode::Mod => {
                        let a = stack.pop_u128()?;
                        let b = stack.pop_u128()?;
                        stack.push_u128(if b == 0 { 0 } else { a % b })?;
                        pc += consumed;
                    }
                    Opcode::Lt => {
                        let a = stack.pop_u128()?;
                        let b = stack.pop_u128()?;
                        stack.push_u128(if a < b { 1 } else { 0 })?;
                        pc += consumed;
                    }
                    Opcode::Gt => {
                        let a = stack.pop_u128()?;
                        let b = stack.pop_u128()?;
                        stack.push_u128(if a > b { 1 } else { 0 })?;
                        pc += consumed;
                    }
                    Opcode::Eq => {
                        let a = stack.pop()?;
                        let b = stack.pop()?;
                        stack.push_u128(if a == b { 1 } else { 0 })?;
                        pc += consumed;
                    }
                    Opcode::IsZero => {
                        let a = stack.pop()?;
                        let is_zero = a.iter().all(|&b| b == 0);
                        stack.push_u128(if is_zero { 1 } else { 0 })?;
                        pc += consumed;
                    }
                    Opcode::And => {
                        let a = stack.pop()?;
                        let b = stack.pop()?;
                        let mut out = [0u8; 32];
                        for i in 0..32 {
                            out[i] = a[i] & b[i];
                        }
                        stack.push(out)?;
                        pc += consumed;
                    }
                    Opcode::Or => {
                        let a = stack.pop()?;
                        let b = stack.pop()?;
                        let mut out = [0u8; 32];
                        for i in 0..32 {
                            out[i] = a[i] | b[i];
                        }
                        stack.push(out)?;
                        pc += consumed;
                    }
                    Opcode::Xor => {
                        let a = stack.pop()?;
                        let b = stack.pop()?;
                        let mut out = [0u8; 32];
                        for i in 0..32 {
                            out[i] = a[i] ^ b[i];
                        }
                        stack.push(out)?;
                        pc += consumed;
                    }
                    Opcode::Not => {
                        let a = stack.pop()?;
                        let mut out = [0u8; 32];
                        for i in 0..32 {
                            out[i] = !a[i];
                        }
                        stack.push(out)?;
                        pc += consumed;
                    }
                    Opcode::Keccak256 => {
                        let offset = stack.pop_usize()?;
                        let size = stack.pop_usize()?;
                        let bytes = memory.read_range(offset, size);
                        stack.push(keccak256(&bytes))?;
                        pc += consumed;
                    }
                    Opcode::CallValue => {
                        stack.push_u128(self.callvalue)?;
                        pc += consumed;
                    }
                    Opcode::CallDataLoad => {
                        let offset = stack.pop_usize()?;
                        let mut word = [0u8; 32];
                        for (i, slot) in word.iter_mut().enumerate() {
                            if let Some(&b) = self.calldata.get(offset.saturating_add(i)) {
                                *slot = b;
                            }
                        }
                        stack.push(word)?;
                        pc += consumed;
                    }
                    Opcode::CallDataSize => {
                        stack.push_u128(self.calldata.len() as u128)?;
                        pc += consumed;
                    }
                    Opcode::Pop => {
                        stack.pop()?;
                        pc += consumed;
                    }
                    Opcode::MLoad => {
                        let offset = stack.pop_usize()?;
                        let word = memory.mload(offset);
                        stack.push(word)?;
                        pc += consumed;
                    }
                    Opcode::MStore => {
                        let offset = stack.pop_usize()?;
                        let value = stack.pop()?;
                        memory.mstore(offset, &value);
                        pc += consumed;
                    }
                    Opcode::MStore8 => {
                        let offset = stack.pop_usize()?;
                        let value = stack.pop()?;
                        memory.mstore8(offset, value[31]);
                        pc += consumed;
                    }
                    Opcode::SLoad => {
                        let key = stack.pop()?;
                        let value = storage.get(&key).copied().unwrap_or([0u8; 32]);
                        stack.push(value)?;
                        pc += consumed;
                    }
                    Opcode::SStore => {
                        let key = stack.pop()?;
                        let value = stack.pop()?;
                        if value == [0u8; 32] {
                            storage.remove(&key);
                        } else {
                            storage.insert(key, value);
                        }
                        pc += consumed;
                    }
                    Opcode::Jump => {
                        let dest = stack.pop_usize()?;
                        if dest >= self.bytecode.len()
                            || self.bytecode[dest] != Opcode::JumpDest as u8
                        {
                            return Err("Invalid JUMP destination (missing JUMPDEST)");
                        }
                        pc = dest;
                    }
                    Opcode::JumpI => {
                        let dest = stack.pop_usize()?;
                        let cond = stack.pop()?;
                        if is_nonzero(&cond) {
                            if dest >= self.bytecode.len()
                                || self.bytecode[dest] != Opcode::JumpDest as u8
                            {
                                return Err("Invalid JUMPI destination (missing JUMPDEST)");
                            }
                            pc = dest;
                        } else {
                            pc += consumed;
                        }
                    }
                    Opcode::Pc => {
                        stack.push_u128(current_pc as u128)?;
                        pc += consumed;
                    }
                    Opcode::MSize => {
                        stack.push_u128(memory.size() as u128)?;
                        pc += consumed;
                    }
                    Opcode::Gas => {
                        stack.push_u128((self.gas_limit - gas_used) as u128)?;
                        pc += consumed;
                    }
                    Opcode::JumpDest => {
                        pc += consumed;
                    }
                    Opcode::Push0
                    | Opcode::Push1
                    | Opcode::Push2
                    | Opcode::Push4
                    | Opcode::Push8
                    | Opcode::Push16
                    | Opcode::Push20
                    | Opcode::Push32 => {
                        // Handled by read_action as Action::PushData
                        pc += consumed;
                    }
                    Opcode::Dup1 => {
                        let val = stack.peek(0)?;
                        stack.push(val)?;
                        pc += consumed;
                    }
                    Opcode::Dup2 => {
                        let val = stack.peek(1)?;
                        stack.push(val)?;
                        pc += consumed;
                    }
                    Opcode::Swap1 => {
                        stack.swap(1)?;
                        pc += consumed;
                    }
                    Opcode::Swap2 => {
                        stack.swap(2)?;
                        pc += consumed;
                    }
                    Opcode::Return => {
                        let offset = stack.pop_usize()?;
                        let size = stack.pop_usize()?;
                        let return_data = memory.read_range(offset, size);
                        return Ok(ExecutionOutcome {
                            stack,
                            memory,
                            storage,
                            return_data,
                            gas_used,
                            reverted: false,
                        });
                    }
                    Opcode::Revert => {
                        let offset = stack.pop_usize()?;
                        let size = stack.pop_usize()?;
                        let return_data = memory.read_range(offset, size);
                        return Ok(ExecutionOutcome {
                            stack,
                            memory,
                            storage,
                            return_data,
                            gas_used,
                            reverted: true,
                        });
                    }
                    Opcode::Invalid => {
                        return Err("Hit INVALID (0xfe) opcode");
                    }
                },
            }
        }

        Ok(ExecutionOutcome {
            stack,
            memory,
            storage,
            return_data: Vec::new(),
            gas_used,
            reverted: false,
        })
    }
}

pub fn is_nonzero(word: &[u8; 32]) -> bool {
    word.iter().any(|&b| b != 0)
}

pub fn decode_u128(word: &[u8; 32]) -> u128 {
    let mut buf = [0u8; 16];
    buf.copy_from_slice(&word[16..]);
    u128::from_be_bytes(buf)
}

pub fn encode_u128(n: u128) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[16..].copy_from_slice(&n.to_be_bytes());
    word
}

pub fn format_word_compact(word: &[u8; 32]) -> Vec<u8> {
    if let Some(first_nonzero) = word.iter().position(|&b| b != 0) {
        word[first_nonzero..].to_vec()
    } else {
        vec![0]
    }
}

pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let digest = Keccak256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_add() {
        // PUSH1 0x02, PUSH1 0x03, ADD -> 5
        let evm = ToyEvm::new(vec![
            Opcode::Push1 as u8,
            0x02,
            Opcode::Push1 as u8,
            0x03,
            Opcode::Add as u8,
        ]);
        let mut stack = evm.run().expect("EVM execution should succeed");
        let top = stack.pop_u128().expect("stack should not be empty");
        assert_eq!(top, 5);
    }

    #[test]
    fn test_sub_and_mul() {
        // In EVM, top of stack is `a` and second item is `b` (SUB computes a - b):
        // PUSH1 3, PUSH1 10, SUB -> 10 - 3 = 7
        // PUSH1 6, MUL -> 6 * 7 = 42
        let evm = ToyEvm::new(vec![
            Opcode::Push1 as u8,
            3,
            Opcode::Push1 as u8,
            10,
            Opcode::Sub as u8,
            Opcode::Push1 as u8,
            6,
            Opcode::Mul as u8,
        ]);
        let mut stack = evm.run().expect("EVM execution should succeed");
        assert_eq!(stack.pop_u128().unwrap(), 42);
    }

    #[test]
    fn test_mstore_mload_and_return() {
        // Store 0x2a (42) at memory offset 0, load it back, then RETURN 32 bytes from offset 0
        let evm = ToyEvm::new(vec![
            Opcode::Push1 as u8,
            0x2a,
            Opcode::Push0 as u8,
            Opcode::MStore as u8,
            Opcode::Push1 as u8,
            32,
            Opcode::Push0 as u8,
            Opcode::Return as u8,
        ]);
        let outcome = evm.execute(Storage::new()).unwrap();
        assert!(!outcome.reverted);
        assert_eq!(outcome.return_data.len(), 32);
        assert_eq!(outcome.return_data[31], 0x2a);
    }

    #[test]
    fn test_sstore_and_sload() {
        // SSTORE(slot = 1, value = 99), then SLOAD(slot = 1)
        let evm = ToyEvm::new(vec![
            Opcode::Push1 as u8,
            99,
            Opcode::Push1 as u8,
            1,
            Opcode::SStore as u8,
            Opcode::Push1 as u8,
            1,
            Opcode::SLoad as u8,
        ]);
        let mut outcome = evm.execute(Storage::new()).unwrap();
        assert_eq!(outcome.stack.pop_u128().unwrap(), 99);
        assert_eq!(outcome.storage.get(&encode_u128(1)), Some(&encode_u128(99)));
    }

    #[test]
    fn test_jump_and_jumpi() {
        // PUSH1 0x06, JUMP, INVALID, JUMPDEST, PUSH1 0x09
        let evm = ToyEvm::new(vec![
            Opcode::Push1 as u8,
            0x04,
            Opcode::Jump as u8,
            Opcode::Invalid as u8,
            Opcode::JumpDest as u8,
            Opcode::Push1 as u8,
            0x09,
        ]);
        let mut stack = evm.run().expect("JUMP over INVALID should succeed");
        assert_eq!(stack.pop_u128().unwrap(), 9);
    }

    #[test]
    fn test_keccak256_opcode() {
        // Store 0x01 at offset 0 (32 bytes), compute KECCAK256(offset=0, size=32)
        let evm = ToyEvm::new(vec![
            Opcode::Push1 as u8,
            0x01,
            Opcode::Push0 as u8,
            Opcode::MStore as u8,
            Opcode::Push1 as u8,
            32,
            Opcode::Push0 as u8,
            Opcode::Keccak256 as u8,
        ]);
        let mut stack = evm.run().unwrap();
        let hash = stack.pop().unwrap();
        assert_eq!(hash, keccak256(&encode_u128(1)));
    }
}