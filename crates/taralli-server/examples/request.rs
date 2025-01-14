use dotenv::dotenv;
use reqwest::Client;
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let port = env::var("SERVER_PORT").expect("SERVER_PORT must be set in .env file");
    let url_str = format!("http://localhost:{}/submit", port);

    let client = Client::new();

    let json_body = json!({
      "proof_request_data": {
        "proving_system_id": "Groth16Bn128",
        "circuit_id": [
          1
        ],
        "proving_system_commitment_id": [
          1
        ],
        "public_inputs": [
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          0,
          33
        ]
      },
      "onchain_proof_request": {
        "proving_time": "0x78",
        "nonce": "0x1c6",
        "token": "0x1c39ba375fab6a9f6e0c01b9f49d488e101c2011",
        "amount": "0x16345785d8a0000",
        "min_reward": "0xb1a2bc2ec50000",
        "market": "0x44863f234b137a395e5c98359d16057a9a1fac55",
        "start_timestamp": 1724426628,
        "minimum_stake": 100000000000000000_u64,
        "meta": {
          "extra_data": "0x0000000000000000000000005615deb798bb3e4dfa0139dfa1b3d433cc23b72f43753b4d0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000020",
          "public_inputs_digest": "0x3a6357012c1a3ae0a17d304c9920310382d968ebcc4b1771f41c6b304205b570"
        },
        "deadline": 1724426688
      },
      "signature": "0x1663ac71cf5052a3e48ed647fa52ba3c6fba831c3bf7e8b910177b092a3dddc529796d7574e9a2c82f6bb0ec62b929b503d1a1e3b7734d01e3d3894f57c20a281c",
      "signer": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266"
    });
    // Send POST request to /submit endpoint
    let response = client.post(url_str).json(&json_body).send().await?;

    // Check the response status
    if response.status().is_success() {
        println!("Request successful!");
        println!("Response: {}", response.text().await?);
    } else {
        println!("Request failed with status: {}", response.status());
    }

    Ok(())
}
