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
