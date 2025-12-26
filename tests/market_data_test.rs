use thalex_rust_sdk::ws_client::WsClient;

#[tokio::test]
async fn test_instruments() {
    let client = WsClient::new_public().await.unwrap();
    let result = client.rpc().market_data().instruments().await;
    assert!(
        result.is_ok(),
        "Market data instruments failed: {:?}",
        result.err()
    );
    client.shutdown("Test complete").await.unwrap();
}
