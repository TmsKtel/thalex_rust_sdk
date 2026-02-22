use thalex_rust_sdk::{types::Environment, ws_client::WsClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Environment::Mainnet;
    // public client
    let client = WsClient::new_public(env).await.unwrap();
    // shutting down the client
    client.wait_for_connection().await;
    println!("Client connected, shutting down...");
    client.shutdown("Done!").await.unwrap();

    let private_client = WsClient::from_env().await.unwrap();
    println!("Private client created from environment variables, shutting down...");
    private_client
        .shutdown("Done with private client!")
        .await
        .unwrap();

    // custom environment
    let custom_env = Environment::Custom("wss://testnet.thalex.com/ws/api/v2".to_string());
    let custom_client = WsClient::new_public(custom_env).await.unwrap();
    println!("Custom client connected, shutting down...");
    custom_client
        .shutdown("Done with custom client!")
        .await
        .unwrap();

    Ok(())
}
