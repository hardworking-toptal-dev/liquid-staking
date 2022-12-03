mod utils;
use std::hash;

use rayon::prelude::*;
use sha2::{Digest, Sha256};
use wasm_bindgen::{prelude::*, JsCast};
// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub struct MinedProof {
    pub nonce: u64,
    hash: String,
    pub success: bool,
}

#[wasm_bindgen]
impl MinedProof {
    #[wasm_bindgen(constructor)]
    pub fn new(nonce: u64, hash: String, success: bool) -> Self {
        Self {
            nonce,
            hash,
            success,
        }
    }
    #[wasm_bindgen(getter)]
    pub fn hash(&self) -> String {
        self.hash.clone()
    }
}

#[wasm_bindgen]
pub fn mine(
    miner_entropy: &str,
    miner_address: &str,
    difficulty: u64,
    start_nonce: u64,
    max_tries: u64,
) -> MinedProof {
    // while loop to find the nonce
    let mut nonce: u64 = start_nonce;
    loop {
        // validate block hash
        let mut hasher = Sha256::new();
        hasher.update(&miner_entropy);
        hasher.update(miner_address);
        hasher.update(nonce.to_le_bytes());
        let result = hasher.finalize();
        let entropy_hash = hex::encode(result);
        let entropy_hash = String::from_utf8(entropy_hash.as_bytes().to_vec()).unwrap();

        // validate difficulty
        let mut difficulty_string = String::new();
        for _ in 0..difficulty {
            difficulty_string.push('0');
        }

        let success = entropy_hash.starts_with(&difficulty_string);

        let is_done = nonce - start_nonce >= max_tries;

        if success || is_done {
            return MinedProof {
                nonce,
                hash: entropy_hash,
                success,
            };
        }
        nonce += 1;
    }
}
