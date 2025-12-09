use thalex_rust_sdk::ws_client::WsClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = WsClient::new();

    client.connect().await?;

    client
        .subscribe("ticker.BTC-PERPETUAL.100ms", |msg| {
            println!("BTC: {msg}");
        })
        .await?;

    client
        .subscribe("ticker.ETH-PERPETUAL.100ms", |msg| {
            println!("ETH: {msg}");
        })
        .await?;

    client.run_forever().await?;

    Ok(())
}
