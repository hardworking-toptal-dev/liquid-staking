# delegated proof of work

## proofs

a proof is a nonce for the next sha256 hash meeting the difficulty requirements set by the liquid staking contract. a proof is a hash of the following: 
* entropy
* miner's bech32 address 
* nonce

miners will search for the next nonce by iteratively until they find a hash that starts with a number of zeros defined by difficulty.

## delegations 

each tendermint block is considered a unit of mining power. when a miner submits a valid proof, the mining power of the validator they delegate to will increase by the number of blocks that have elapsed since the last valid proof was submitted. 

delegations are incrementally and periodically rebalanced, with their stake proportionally weighted according to their mining power.

## mining rewards 

at present, 16% all new staking rewards collected by the contract are sent to the last miner to submit a valid proof.

## mining difficulty 

at present, if a proof takes more than 5 minutes to mine, the difficulty will decrease by 1.

if a proof takes less than 20 seconds to mine, the difficulty will increase by 1.

## mining 

### browser prototype 
the browser prototype is a rust wasm module that can be used to mine proofs in the browser. a keplr wallet popup will occur every time you mine a proof. you can find this at [https://wetjoe.netlify.app/](https://wetjoe.netlify.app/).

### cli prototype
the cli prototype is a rust script that can be used to mine proofs in the terminal. you can find this at [./packages/dpow-miner-cli](./packages/dpow-miner-cli). 

#### prerequisites
* rust installed (https://www.rust-lang.org/tools/install)
* `joed` installed (https://github.com/Reecepbcups/joe)
* wallet added to `joed` keychain (`joed keys add miner --interactive`)
* joe tokens (ask [joe](https://twitter.com/CosmosDefi) for some after you give him a follow and like all of his youtube videos)
* `jq` installed (https://stedolan.github.io/jq/download/)

run as shown below:

see in [deploy/run-miner.sh](./deploy/run-miner.sh)

```bash
export VALIDATOR_ADDRESS=joevaloper<YOUR_PREFERRED_VALIDATOR>
export MINER_ADDRESS=<YOUR_ADDRESS_IN_JOED>
export CONTRACT_ADDRESS=joe18yn206ypuxay79gjqv6msvd9t2y49w4fz8q7fyenx5aggj0ua37qnv0qf3
export RPC_URL=https://joe-rpc.polkachu.com:443
# number between 1 and 1000. 1000 is fastest.
export SPEED=500
export THREAD_COUNT=2
cargo run ./packages/dpow-miner-cli --release
```



----------------------------------------------------------------------------------



----------------------------------------------------------------------------------



----------------------------------------------------------------------------------


----------------------------------------------------------------------------------
disclaimer

All software in the repository is considered highly experimental. The author considers JOE tokens to be non-monetary, for testing purposes only. The author is not responsible for any losses incurred by using this software. All software is provided as is, without warranty of any kind, express or implied, including but not limited to the warranties of merchantability, fitness for a particular purpose and noninfringement. In no event shall the authors or copyright holders be liable for any claim, damages or other liability, whether in an action of contract, tort or otherwise, arising from, out of or in connection with the software or the use or other dealings in the software. 