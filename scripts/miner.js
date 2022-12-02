// Write a function called `mine` to: 
// * take a bech32 address and an entropy string as arguments
// * use browser native window.crypto api to generate a random u64 nonce as little endian bytes
// * use browser native window.crypto api to generate hash updated with entropy, the bech32 address, and the nonce
async function mine(address, difficulty, entropy) {
    // validate difficulty
    let difficultyString = "";
    for (let i = 0; i < difficulty; i++) {
        difficultyString += "0";
    }

    // generate random u64 nonce as little endian bytes
    let nonce = window.crypto.getRandomValues(new Uint8Array(8));

    var dataView = new DataView(nonce.buffer);
    var integerNonce = dataView.getBigUint64(0, true);

    console.log('nonce', integerNonce);

    let entropyBytes = new TextEncoder().encode(entropy);
    let addressBytes = new TextEncoder().encode(address);
    let dataBytes = new Uint8Array([...entropyBytes, ...addressBytes, ...nonce]);
    // use browser native window.crypto api to generate hash updated with entropy, the bech32 address, and the nonce
    let entropyHash = await window.crypto.subtle.digest("SHA-256", dataBytes).then(function (hash) {
      // convert hash to hex string
      let entropyHash = Array.from(new Uint8Array(hash)).map(b => b.toString(16).padStart(2, '0')).join('');
      return entropyHash;
    });

    // validate block hash
    if (!entropyHash.startsWith(difficultyString)) {
        return "block hash does not meet difficulty requirement";
    }

    // return nonce
    return nonce;
}
