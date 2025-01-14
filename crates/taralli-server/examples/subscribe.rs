use alloy::primitives::Address;
use dotenv::dotenv;
use futures::StreamExt;
use reqwest::Client;
use std::env;
use url::Url; // Add this import at the top of the file

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let port = env::var("SERVER_PORT").expect("SERVER_PORT must be set in .env file");

    // Generate a random Ethereum address
    let random_address = Address::random();

    // Create a client
    let client = Client::new();

    // Construct the URL with the query parameter
    let url_str = format!("http://localhost:{}/subscribe", port);
    let mut url = Url::parse(&url_str)?;
    url.query_pairs_mut()
        .append_pair("user-id", &random_address.to_string());

    println!("New Address: {}", random_address);
    println!("Connecting to: {}", url);

    // Send GET request
    let response = client.get(url).send().await?;

    // Check if the connection was successful
    if response.status().is_success() {
        println!("Connected successfully. Listening for events...");

        // Stream the response body
        let mut stream = response.bytes_stream();
        while let Some(item) = stream.next().await {
            match item {
                Ok(bytes) => {
                    // Process the received data
                    println!("Received: {:?}", String::from_utf8_lossy(&bytes));
                }
                Err(e) => {
                    eprintln!("Error receiving data: {:?}", e);
                    break;
                }
            }
        }
    } else {
        println!("Failed to connect: {:?}", response.status());
    }

    Ok(())
}
