use anyhow::{Context, Ok, Result};
use async_std::io::ReadExt;
use futures::future;
use pfc_steak::hub::MinerParamsResponse;
use rand::distributions::Alphanumeric;
use rand::Rng;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::error::Error;
use std::hash;
use std::io::Write;
use std::ops::DerefMut;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tokio;
pub struct State {
    pub miner_params: MinerParamsResponse,
    pub tx_in_flight: bool,
    pub miner_params_loaded: bool,
}

#[tokio::main]
async fn main() {
    // while loop to find the nonce
    let state = Arc::new(Mutex::new(State {
        miner_params: MinerParamsResponse {
            entropy: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            difficulty: 0_u64.into(),
        },
        tx_in_flight: false,
        miner_params_loaded: false,
    }));

    // spawn thread with infinite loop inside
    // that will update the miner entropy and difficulty
    // every X seconds
    let polling_state = state.clone();
    std::thread::spawn(move || -> Result<()> {
        loop {
            let miner_params_res = poll_miner_params()?;
            println!("Miner Params: {:?}", miner_params_res);
            polling_state.lock().unwrap().miner_params = miner_params_res;
            polling_state.lock().unwrap().miner_params_loaded = true;
            std::thread::sleep(std::time::Duration::from_secs(14));
        }
    });

    // spawn thread with infinite loop inside
    // that will update the entropy
    // every X seconds
    let entropy = tokio::spawn(async move {
        loop {
            if let Err(e) = update_entropy() {
                println!("Error updating entropy: {}", e);
            }
            println!("updated entropy");
            std::thread::sleep(std::time::Duration::from_secs(1200));
        }
    });

    fn create_tokio_handler(
        i: u64,
        state: Arc<Mutex<State>>,
    ) -> tokio::task::JoinHandle<Result<(), anyhow::Error>> {
        tokio::spawn(async move {
            let mut last_nonce = i * 100_000_000_000;
            while 1 < 2 {
                let proof = mine(last_nonce.clone() + 1, state.clone()).unwrap();
                last_nonce = proof.nonce;
                if proof.success && !state.lock().unwrap().tx_in_flight {
                    state.lock().unwrap().tx_in_flight = true;
                    println!("Submitting proof: {}", proof.hash);
                    submit_proof(proof).unwrap();
                    state.lock().unwrap().tx_in_flight = false;
                }
            }
            Ok(())
        })
    }

    let mut handlers = vec![];
    for i in 0..get_thread_count_as_int() {
        handlers.push(create_tokio_handler(i, state.clone()));
    }

    handlers.push(entropy);

    future::join_all(handlers).await;
}

pub fn poll_miner_params() -> Result<MinerParamsResponse> {
    let miner_params_query = pfc_steak::hub::QueryMsg::MinerParams {};
    let joed_cosmwasm_query = Command::new("joed")
        .arg("q")
        .arg("wasm")
        .arg("contract-state")
        .arg("smart")
        .arg(get_contract_address())
        .arg(format!(
            "{}",
            serde_json::to_string(&miner_params_query).context("serializing miner params query")?
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
        serde_json::from_str(&String::from_utf8_lossy(&joed_cosmwasm_query.stdout))
            .context("parsing json from contract-state query")?;
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
    let query_response: pfc_steak::hub::MinerParamsResponse = serde_json::from_value(
        parsed_json
            .get("data")
            .context("getting data field from miner params json")?
            .to_owned(),
    )
    .context("parsing miner params json")?;
    Ok(query_response)
}

pub fn submit_proof(proof: MinedProof) -> Result<()> {
    // wait 3 seconds
    std::thread::sleep(std::time::Duration::from_secs(3));
    let joed_cosmwasm_tx_result = Command::new("joed")
        .arg("tx")
        .arg("wasm")
        .arg("execute")
        .arg(get_contract_address())
        .arg(
            serde_json::to_string(&pfc_steak::hub::ExecuteMsg::SubmitProof {
                nonce: proof.nonce.into(),
                validator: get_validator_address(),
            })
            .unwrap(),
        )
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
        .arg("text")
        .output()
        .expect("failed to execute process");
    println!("status: {}", joed_cosmwasm_tx_result.status.to_string());
    println!(
        "stdout: {}",
        String::from_utf8_lossy(&joed_cosmwasm_tx_result.stdout)
    );
    println!(
        "stderr: {}",
        String::from_utf8_lossy(&joed_cosmwasm_tx_result.stderr)
            .split('\n')
            .collect::<Vec<&str>>()[0]
    );
    Ok(())
}

// generate entropy and execute UpdateEntropy on joed
pub fn update_entropy() -> Result<String, anyhow::Error> {
    println!("Updating entropy...");
    std::thread::sleep(std::time::Duration::from_secs(3));
    let entropy: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    println!("entropy: {}", entropy);
    let joed_cosmwasm_tx_result = Command::new("joed")
        .arg("tx")
        .arg("wasm")
        .arg("execute")
        .arg(get_contract_address())
        .arg(
            serde_json::to_string(&pfc_steak::hub::ExecuteMsg::UpdateEntropy {
                entropy: entropy.clone(),
            })
            .context("serializing UpdateEntropy message")?,
        )
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
        .context("executing UpdateEntropy tx")?;
    let joed_cosmwasm_tx_result = if joed_cosmwasm_tx_result.status.success() {
        let mut p = std::process::Command::new("jq")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let output = String::from_utf8_lossy(&joed_cosmwasm_tx_result.stdout);
        write!(p.stdin.as_mut().unwrap(), "{}", output).unwrap();
        p.wait_with_output()?
    } else {
        joed_cosmwasm_tx_result
    };
    println!("updated entropy");
    println!("status: {}", joed_cosmwasm_tx_result.status.to_string());
    println!(
        "stdout: {}",
        String::from_utf8_lossy(&joed_cosmwasm_tx_result.stdout)
    );
    println!(
        "stderr: {}",
        String::from_utf8_lossy(&joed_cosmwasm_tx_result.stderr)
            .split('\n')
            .collect::<Vec<&str>>()[0]
    );
    Ok(entropy)
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
pub fn get_speed_as_int() -> u64 {
    std::env::var("SPEED").unwrap().parse::<u64>().unwrap()
}
pub fn get_thread_count_as_int() -> u64 {
    std::env::var("THREAD_COUNT")
        .unwrap()
        .parse::<u64>()
        .unwrap()
}

pub fn mine(start_nonce: u64, miner_params: Arc<Mutex<State>>) -> Result<MinedProof> {
    let miner_address: String = get_miner_address();
    let mut nonce: u64 = start_nonce;
    // speed of 1_000 = 0 second delay every 100k hashes
    // speed of 500 = 0.05 second delay every 50k hashes
    // speed of 100 = 0.09 second delay every 10k hashes
    // speed of 1 = ~0.1 second delay every 100 hashes
    let delay = std::time::Duration::from_millis((1_000 - get_speed_as_int()) / 10);
    let delay_increment = get_speed_as_int() * 100;
    loop {
        if nonce % delay_increment == 0 && !delay.is_zero() {
            std::thread::sleep(delay);
            println!("nonce: {}", nonce);
        }
        if miner_params.lock().unwrap().tx_in_flight
            || !miner_params.lock().unwrap().miner_params_loaded
        {
            // println!("tx in flight, waiting");
            std::thread::sleep(std::time::Duration::from_secs(7));
            continue;
        }
        // validate block hash
        let mut hasher = Sha256::new();
        let miner_params = miner_params.lock().unwrap().miner_params.clone();
        // print!("entropy: {} ", miner_params.entropy);
        // println!("difficulty: {}", miner_params.difficulty);
        hasher.update(&miner_params.entropy);
        hasher.update(&miner_address);
        hasher.update(nonce.to_le_bytes());
        let result = hasher.finalize();
        let entropy_hash = hex::encode(result);
        let entropy_hash = String::from_utf8(entropy_hash.as_bytes().to_vec())
            .context("converting entropy hash to string")?;

        // validate difficulty
        let mut difficulty_string = String::new();
        for _ in 0..miner_params.difficulty.u64() {
            difficulty_string.push("0".to_string().chars().next().unwrap());
        }
        let success = entropy_hash.starts_with(&difficulty_string.clone());

        if success {
            // print miner params
            // println!("Miner Params: {:?}", miner_params);
            // print nonce
            println!("hash {}", entropy_hash);
            println!("difficulty string {:?}", difficulty_string);
            println!("Nonce: {}", nonce);
            return Ok(MinedProof {
                nonce,
                hash: entropy_hash,
                success,
            });
        }
        nonce = nonce.wrapping_add(1u64);
    }
}
