use log::Level::Info;
use serde_json::json;
use simple_logger::init_with_level;
use thalex_rust_sdk::ws_client::WsClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_with_level(Info).unwrap();

    let client = WsClient::connect_default().await.unwrap();

    let response = client
        .call_rpc("public/instruments", json!({}))
        .await
        .unwrap();
    println!("RPC response: {response}");

    let _ = client
        .subscribe("ticker.BTC-PERPETUAL.100ms", |msg| {
            println!("BTC tick: {msg}");
        })
        .await;

    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    client.shutdown().await.unwrap();
    Ok(())
}
