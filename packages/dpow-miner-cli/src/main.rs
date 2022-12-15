use pfc_steak::hub::MinerParamsResponse;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::hash;
use std::process::Command;

fn main() {
    mine();
    // let output = Command::new("ls")
    //     .arg("file.txt")
    //     .output()
    //     .expect("failed to execute process");

    // println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    // println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // assert!(output.status.success());
}

pub fn poll_miner_params() -> MinerParamsResponse {
    let miner_params_query = pfc_steak::hub::QueryMsg::MinerParams {};
    let joed_cosmwasm_query = Command::new("joed")
        .arg("q")
        .arg("wasm")
        .arg("contract-state")
        .arg("smart")
        .arg(get_contract_address())
        .arg(format!(
            "{}",
            serde_json::to_string(&miner_params_query).unwrap()
        ))
        .arg("--node")
        .arg(get_rpc_url())
        .arg("--chain-id")
        .arg("joe-1")
        .arg("--output")
        .arg("json")
        .output()
        .expect("failed to execute process");
    let parsed_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&joed_cosmwasm_query.stdout)).unwrap();
    println!("status: {}", joed_cosmwasm_query.status);
    println!(
        "stdout: {}",
        String::from_utf8_lossy(&joed_cosmwasm_query.stdout)
    );
    println!(
        "stderr: {}",
        String::from_utf8_lossy(&joed_cosmwasm_query.stderr)
    );
    println!(
        "stdout: {}",
        String::from_utf8_lossy(&joed_cosmwasm_query.stdout)
    );
    let query_response: pfc_steak::hub::MinerParamsResponse =
        serde_json::from_value(parsed_json.get("data").unwrap().to_owned()).unwrap();
    println!("Miner Params: {:?}", query_response);
    query_response
}

pub fn submit_proof(proof: MinedProof) {
    let joed_cosmwasm_tx_result = Command::new("joed")
        .arg("tx")
        .arg("wasm")
        .arg("execute")
        .arg(get_contract_address())
        .arg(format!(
            "{}",
            serde_json::to_string(&pfc_steak::hub::ExecuteMsg::SubmitProof {
                nonce: proof.nonce.into(),
                validator: get_validator_address()
            })
            .unwrap()
        ))
        .arg("--from")
        .arg(get_miner_address())
        .arg("--node")
        .arg(get_rpc_url())
        .arg("--chain-id")
        .arg("joe-1")
        .arg("--gas")
        .arg("auto")
        .arg("--gas-adjustment")
        .arg("1.5")
        .arg("--gas-prices")
        .arg("0.025ujoe")
        .arg("--fees")
        .arg("0.025ujoe")
        .arg("--broadcast-mode")
        .arg("block")
        .arg("-y")
        .arg("--output")
        .arg("json")
        .output()
        .expect("failed to execute process");
    println!("status: {}", joed_cosmwasm_tx_result.status);
    println!(
        "stdout: {}",
        String::from_utf8_lossy(&joed_cosmwasm_tx_result.stdout)
    );
    println!(
        "stderr: {}",
        String::from_utf8_lossy(&joed_cosmwasm_tx_result.stderr)
    );
}

pub struct MinedProof {
    pub nonce: u64,
    hash: String,
    pub success: bool,
}

pub fn get_miner_address() -> String {
    std::env::var("MINER_ADDRESS").unwrap().to_string()
}
pub fn get_validator_address() -> String {
    std::env::var("VALIDATOR_ADDRESS").unwrap().to_string()
}
pub fn get_contract_address() -> String {
    std::env::var("CONTRACT_ADDRESS").unwrap().to_string()
}
pub fn get_rpc_url() -> String {
    std::env::var("RPC_URL").unwrap().to_string()
}

pub fn mine() -> MinedProof {
    let miner_address: String = get_miner_address();
    let mut miner_entropy: String =
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string();
    let mut difficulty: u64 = 0_u64;
    let start_nonce: u64 = 0_u64;
    let mut last_updated_params = 0;
    // while loop to find the nonce
    let mut nonce: u64 = start_nonce;
    loop {
        // if 30 seconds has passed, query miner params again
        if last_updated_params + 30
            < std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        {
            last_updated_params = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let miner_params = poll_miner_params();
            difficulty = miner_params.difficulty.u64();
            miner_entropy = miner_params.entropy.to_string();
        }

        // validate block hash
        let mut hasher = Sha256::new();
        hasher.update(&miner_entropy);
        hasher.update(&miner_address);
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

        if success {
            submit_proof(MinedProof {
                nonce,
                hash: entropy_hash,
                success,
            });
        }
        nonce = nonce.wrapping_add(1u64);
    }
}
