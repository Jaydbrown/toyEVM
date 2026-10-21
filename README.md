# Jay's Toy EVM Node (`toyEVM`)

A lightweight Ethereum Virtual Machine (EVM) bytecode execution engine, JSON-RPC 2.0 node interface, and peer-to-peer (P2P) gossip network written in Rust.

---

## Overview

**Jay's Toy EVM Node** (`toyEVM`) is a modular, from-scratch implementation of core Ethereum execution and networking primitives in Rust. It models the end-to-end lifecycle of Ethereum execution and block production—from 256-bit stack evaluation, word-aligned linear memory, persistent key-value storage, and gas metering to transaction encoding, Keccak-256 block mining, and decentralized P2P gossiping.

The system is split into four primary layers:
1. **EVM Execution Engine:** A 256-bit stack-based virtual machine with linear memory, persistent contract storage, control-flow jumps (`JUMP`/`JUMPI`/`JUMPDEST`), and per-opcode gas metering.
2. **Transaction Layer:** Binary encoding and decoding of Ethereum transactions (`EthTx`), supporting both value transfers (`to = Some(addr)`) and contract creation (`to = None`).
3. **JSON-RPC Server:** A JSON-RPC 2.0 server (`jsonrpsee`) exposing endpoints for EVM bytecode execution (`eth_call` / `eval_bytecode`), raw transaction encoding/decoding, block mining, and node/txpool inspection.
4. **P2P Gossip Network:** An asynchronous TCP networking layer (`tokio`) enabling multiple running nodes to discover bootnodes, gossip transactions and blocks, and maintain synchronized transaction pools.

---

## Features

### 1. Ethereum Virtual Machine (`ToyEvm`)
- **256-bit Word Stack (`Stack`):** Capped at 1024 words per the Ethereum Yellow Paper specification.
- **Word-Aligned Linear Memory (`Memory`):** Byte-addressable memory expanding dynamically in 32-byte word increments (`MLOAD`, `MSTORE`, `MSTORE8`, `MSIZE`).
- **Persistent Contract Storage (`Storage`):** 32-byte key to 32-byte value storage (`SLOAD`, `SSTORE`) with zero-value slot clearing.
- **Supported Opcodes (`Opcode`):**
  - **Stop & Arithmetic:** `STOP` (`0x00`), `ADD` (`0x01`), `MUL` (`0x02`), `SUB` (`0x03`), `DIV` (`0x04`), `MOD` (`0x06`).
  - **Comparison & Bitwise:** `LT` (`0x10`), `GT` (`0x11`), `EQ` (`0x14`), `ISZERO` (`0x15`), `AND` (`0x16`), `OR` (`0x17`), `XOR` (`0x18`), `NOT` (`0x19`).
  - **Hashing:** `KECCAK256` (`0x20`).
  - **Call Context:** `CALLVALUE` (`0x34`), `CALLDATALOAD` (`0x35`), `CALLDATASIZE` (`0x36`).
  - **Stack, Memory, Storage & Flow:** `POP` (`0x50`), `MLOAD` (`0x51`), `MSTORE` (`0x52`), `MSTORE8` (`0x53`), `SLOAD` (`0x54`), `SSTORE` (`0x55`), `JUMP` (`0x56`), `JUMPI` (`0x57`), `PC` (`0x58`), `MSIZE` (`0x59`), `GAS` (`0x5a`), `JUMPDEST` (`0x5b`).
  - **Push, Dup & Swap:** `PUSH0` (`0x5f`), `PUSH1`..`PUSH32` (`0x60`..`0x7f`), `DUP1` (`0x80`), `DUP2` (`0x81`), `SWAP1` (`0x90`), `SWAP2` (`0x91`).
  - **Termination:** `RETURN` (`0xf3`), `REVERT` (`0xfd`), `INVALID` (`0xfe`).

### 2. Transaction Serialization (`EthTx`)
- Models Ethereum transaction fields: `chain_id`, `nonce`, `gas_price`, `gas_limit`, `to` (20-byte address or `None` for contract deployment), `value` (in wei), and `data` (calldata / initcode).
- Big-endian wire encoding and boundary-checked decoding.

### 3. Block Header, Keccak-256 & Mining
- 140-byte `BlockHeader` layout (`parent_hash`, `state_root`, `transactions_root`, `number`, `gas_limit`, `gas_used`, `timestamp`, `difficulty`, `nonce`).
- Keccak-256 (`sha3::Keccak256`) header hashing and Merkle root calculation (`compute_merkle_root`).
- Configurable proof-of-work mining loop (`mine` and `generate`).

### 4. JSON-RPC 2.0 Server & P2P Gossip Networking
- Default Ethereum JSON-RPC port `8545` and P2P bootnode port `30303` with automatic port fallback when running multiple local nodes.
- Deduplicated transaction and block gossip over newline-delimited JSON TCP streams.

---

## Project Structure

```
src/
├── lib.rs                  # ToyEvm execution engine, Keccak-256 helper, and unit tests
├── main.rs                 # Node entrypoint, CLI argument parser, and RPC/P2P runner
├── opcode.rs               # EVM Opcode enum definitions and gas schedule
├── executionStack.rs       # 256-bit Stack, linear Memory, and key-value Storage
├── transaction.rs          # EthTx transaction model
├── blockchain.rs           # Block and Blockchain state management
├── rpc/
│   ├── mod.rs              # Re-exports all RPC traits and server implementations
│   ├── state.rs            # eth_call and eval_bytecode endpoints
│   ├── raw_transactions.rs # decoderawtransaction and createrawtransaction
│   ├── node.rs             # getblockchaininfo, getnetworkinfo, getmempoolinfo
│   └── mining/
│       ├── mod.rs          # Re-exports mining and Keccak-256 Merkle utilities
│       ├── merkle.rs       # keccak256_hash and compute_merkle_root logic
│       └── mining.rs       # BlockHeader, getblocktemplate, submitblock, generate
└── p2p/
    ├── mod.rs              # Re-exports Swarm and P2pMessage
    ├── message.rs          # P2pMessage wire packet enum
    ├── peer.rs             # TCP reader and writer tasks
    └── swarm.rs            # Peer pool, broadcast manager, and deduplication
```

---

## Installation & Build

```bash
cargo check
cargo build
cargo test
```

---

## Running the Node

**Terminal 1 (Bootnode):**
```bash
cargo run
```
- Binds P2P to `127.0.0.1:30303` and JSON-RPC to `http://127.0.0.1:8545`.

**Terminal 2 (Peer Node):**
```bash
cargo run
```
- Automatically selects the next free ports (`30304` for P2P, `8546` for RPC) and connects to the bootnode at `127.0.0.1:30303`.

---

## JSON-RPC API Examples

### 1. `eth_call` / `eval_bytecode`
Executes EVM bytecode (`6002600301` = `PUSH1 0x02 PUSH1 0x03 ADD`).

```bash
curl -X POST http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_call",
    "params": {
      "bytecode_hex": "6002600301"
    },
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true,
    "reverted": false,
    "gas_used": 9,
    "return_data": "",
    "final_stack": ["05"],
    "storage": {},
    "error": null
  },
  "id": 1
}
```

### 2. `createrawtransaction`
```bash
curl -X POST http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "createrawtransaction",
    "params": {
      "chain_id": 1337,
      "nonce": 0,
      "gas_price": 1000000000,
      "gas_limit": 21000,
      "to_hex": "1111111111111111111111111111111111111111",
      "value": 1000000000000000000,
      "data_hex": ""
    },
    "id": 1
  }'
```
