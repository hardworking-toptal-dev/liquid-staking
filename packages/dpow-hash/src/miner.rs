use crate::proof::{bech32_encode_hash, hash_nonce};
use anyhow::Result;
use rand::{self, Rng};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MinerError {
    #[error("difficulty is too low. {}", .input_difficulty)]
    InsufficientDifficulty { input_difficulty: u32 },
}

// function to search for a valid proof
// of work for a given header hash
pub fn search_pow(header_hash: &[u8], difficulty: u32) -> Result<(String, [u8; 32]), MinerError> {
    // throw error if difficulty is lower than 2
    if difficulty < 2 {
        return Err(MinerError::InsufficientDifficulty {
            input_difficulty: difficulty,
        });
    }
    let mut keyword = "j".to_string();
    for _ in 0..difficulty {
        keyword += "0";
    }
    keyword += "e";
    let keyword = "j0e".to_string();
    let keyword = "dpow1".to_string() + &keyword;
    loop {
        let nonce = gen_nonce_bytes();
        let hash_proof = hash_nonce(&header_hash, &nonce);
        let bech32_hash = bech32_encode_hash("dpow", &hash_proof);
        if bech32_hash.starts_with(keyword.as_str()) {
            return Ok((bech32_hash, nonce));
        }
    }
}

pub fn gen_nonce_bytes() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let mut nonce = [0u8; 32];
    rng.fill(&mut nonce);
    nonce
}

// function to generate proof
#[cfg(test)]
mod tests {
    use subtle_encoding::{hex, Encoding};

    use crate::proof::hash_header;

    use super::*;

    #[test]
    fn it_works() {
        let hash = hash_header("hello world".as_bytes());
        let start_time = std::time::Instant::now();
        let (bech32_hash, nonce) = search_pow(&hash, 1).unwrap();
        let hex_nonce = hex::encode(nonce);
        let base64_nonce = subtle_encoding::base64::encoder()
            .encode_to_string(nonce)
            .unwrap();
        let str_nonce = String::from_utf8(hex_nonce).unwrap();
        let substr = &bech32_hash[0..=7];
        assert_eq!(&substr, &"dpow1j0e");
        let end_time = std::time::Instant::now();
        let time_elapsed_seconds = end_time.duration_since(start_time).as_secs_f64();
        println!("{}", time_elapsed_seconds);
    }
}
