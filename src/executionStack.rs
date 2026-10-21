use std::collections::HashMap;

pub const MAX_STACK_DEPTH: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stack {
    pub stack: Vec<[u8; 32]>,
}

impl Stack {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn push(&mut self, word: [u8; 32]) -> Result<(), &'static str> {
        if self.stack.len() >= MAX_STACK_DEPTH {
            return Err("EVM stack overflow (max 1024 words)");
        }
        self.stack.push(word);
        Ok(())
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        if bytes.len() > 32 {
            return Err("Push data exceeds 32-byte EVM word size");
        }
        let mut word = [0u8; 32];
        let start = 32 - bytes.len();
        word[start..].copy_from_slice(bytes);
        self.push(word)
    }

    pub fn push_u128(&mut self, val: u128) -> Result<(), &'static str> {
        let mut word = [0u8; 32];
        word[16..].copy_from_slice(&val.to_be_bytes());
        self.push(word)
    }

    pub fn pop(&mut self) -> Result<[u8; 32], &'static str> {
        self.stack.pop().ok_or("EVM stack underflow")
    }

    pub fn pop_u128(&mut self) -> Result<u128, &'static str> {
        let word = self.pop()?;
        let mut buf = [0u8; 16];
        buf.copy_from_slice(&word[16..]);
        Ok(u128::from_be_bytes(buf))
    }

    pub fn pop_usize(&mut self) -> Result<usize, &'static str> {
        let val = self.pop_u128()?;
        usize::try_from(val).map_err(|_| "EVM word exceeds usize bounds")
    }

    pub fn top(&self) -> Result<&[u8; 32], &'static str> {
        self.stack.last().ok_or("EVM stack underflow")
    }

    pub fn peek(&self, depth: usize) -> Result<[u8; 32], &'static str> {
        if depth >= self.stack.len() {
            return Err("EVM stack underflow");
        }
        Ok(self.stack[self.stack.len() - 1 - depth])
    }

    pub fn swap(&mut self, depth: usize) -> Result<(), &'static str> {
        if depth == 0 || depth >= self.stack.len() {
            return Err("EVM stack underflow");
        }
        let top_idx = self.stack.len() - 1;
        let target_idx = top_idx - depth;
        self.stack.swap(top_idx, target_idx);
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Memory {
    pub data: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn ensure_capacity(&mut self, offset: usize, size: usize) {
        if size == 0 {
            return;
        }
        let needed = offset.saturating_add(size);
        // EVM memory expands in 32-byte word multiples
        let aligned = needed.div_ceil(32) * 32;
        if aligned > self.data.len() {
            self.data.resize(aligned, 0);
        }
    }

    pub fn mstore(&mut self, offset: usize, word: &[u8; 32]) {
        self.ensure_capacity(offset, 32);
        self.data[offset..offset + 32].copy_from_slice(word);
    }

    pub fn mstore8(&mut self, offset: usize, byte: u8) {
        self.ensure_capacity(offset, 1);
        self.data[offset] = byte;
    }

    pub fn mload(&mut self, offset: usize) -> [u8; 32] {
        self.ensure_capacity(offset, 32);
        let mut word = [0u8; 32];
        word.copy_from_slice(&self.data[offset..offset + 32]);
        word
    }

    pub fn read_range(&mut self, offset: usize, size: usize) -> Vec<u8> {
        if size == 0 {
            return Vec::new();
        }
        self.ensure_capacity(offset, size);
        self.data[offset..offset + size].to_vec()
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

pub type Storage = HashMap<[u8; 32], [u8; 32]>;
