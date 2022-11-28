// mining feature flag
use sha2::{Digest, Sha256};
use subtle_encoding::bech32;

pub fn hash_nonce(header_hash: &[u8], nonce: &[u8]) -> Vec<u8> {
    let mut hasher: Sha256 = Sha256::new();
    hasher.update(header_hash);
    hasher.update(nonce);
    let finalized = hasher.finalize();
    finalized.to_vec()
}

// function that creates a header hash from a header
pub fn hash_header(header: &[u8]) -> Vec<u8> {
    let mut hasher: Sha256 = Sha256::new();
    hasher.update(header);
    let finalized = hasher.finalize();
    finalized.to_vec()
}

// function that bech32 encodes a sha256 hash
pub fn bech32_encode_hash(hrp: &str, hash: &[u8]) -> String {
    let hash = hash.to_vec();
    bech32::encode(hrp, hash)
}
