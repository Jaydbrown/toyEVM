use sha3::{Digest, Keccak256};

pub fn keccak256_hash(data: &[u8]) -> [u8; 32] {
    let digest = Keccak256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

pub fn compute_merkle_root(mut hashes: Vec<[u8; 32]>) -> Result<[u8; 32], &'static str> {
    if hashes.is_empty() {
        return Err("Cannot compute transactions root of empty transaction list");
    }

    while hashes.len() > 1 {
        if hashes.len() % 2 != 0 {
            let last = *hashes.last().unwrap();
            hashes.push(last);
        }

        let mut next_level = Vec::with_capacity(hashes.len() / 2);
        for chunk in hashes.chunks_exact(2) {
            let mut combined = [0u8; 64];
            combined[..32].copy_from_slice(&chunk[0]);
            combined[32..].copy_from_slice(&chunk[1]);
            next_level.push(keccak256_hash(&combined));
        }

        hashes = next_level;
    }

    Ok(hashes[0])
}
