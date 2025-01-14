const fs = require("fs");

function toHexString(byteArray) {
  //const chars = new Buffer(byteArray.length * 2);
  const chars = new Uint8Array(byteArray.length * 2);
  const alpha = "a".charCodeAt(0) - 10;
  const digit = "0".charCodeAt(0);

  let p = 0;
  for (let i = 0; i < byteArray.length; i++) {
    let nibble = byteArray[i] >>> 4;
    chars[p++] = nibble > 9 ? nibble + alpha : nibble + digit;
    nibble = byteArray[i] & 0xf;
    chars[p++] = nibble > 9 ? nibble + alpha : nibble + digit;
  }

  //return chars.toString('utf8');
  return String.fromCharCode.apply(null, chars);
}

const rawJson = JSON.parse(fs.readFileSync("./raw-verification-data.json"));
console.log(rawJson);

function convertRawJsonToHex(rawJson) {
  return {
    verification_data_commitment: {
      proof_commitment: toHexString(
        rawJson.verification_data_commitment.proof_commitment
      ),
      pub_input_commitment: toHexString(
        rawJson.verification_data_commitment.pub_input_commitment
      ),
      proving_system_aux_data_commitment: toHexString(
        rawJson.verification_data_commitment.proving_system_aux_data_commitment
      ),
      proof_generator_addr: toHexString(
        rawJson.verification_data_commitment.proof_generator_addr
      ),
    },
    batch_merkle_root: toHexString(rawJson.batch_merkle_root),
    batch_inclusion_proof: {
      merkle_path: rawJson.batch_inclusion_proof.merkle_path.map((x) =>
        toHexString(x)
      ),
    },
    verification_data_batch_index: rawJson.verification_data_batch_index,
    // ...
  };
}

fs.writeFileSync(`verification-data.json`, JSON.stringify(convertRawJsonToHex(rawJson)));
