delegated proof of work

proofs

a proof is a nonce for the next sha256 hash of a virtual block header within the liquid staking contract. 
the virtual block header is a concatenation of the following fields: 
* the block height
* the timestamp
* the total amount of native token staked
* the previous nonce
* the previous header hash

miners will search for the next nonce by appending random bytes to the header and hashing it until they find a hash that, when bech32 encoded, starts with a certain keyword (e.g. "j0e"). 

delegations 

all new delegations will be sent to the validator selected by the last miner to submit a valid proof. thus, the validators who mine the most proofs will receive largest delegations.

mining rewards 

a percentage of all new staking rewards collected by the contract will be sent to the last miner to submit a valid proof.


