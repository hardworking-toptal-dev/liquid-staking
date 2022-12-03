delegated proof of work

proofs

a proof is a nonce for the next sha256 hash meeting the difficulty requirements set by the liquid staking contract. a proof is a hash of the following: 
* entropy
* miner's bech32 address 
* nonce

miners will search for the next nonce by iteratively until they find a hash that starts with a number of zeros defined by difficulty.

delegations 

all new delegations will be sent to the validator selected by the last miner to submit a valid proof. thus, the validators who mine the most proofs will receive largest delegations.

mining rewards 

a percentage of all new staking rewards collected by the contract will be sent to the last miner to submit a valid proof.


