use thalex_rust_sdk::{
    models::{CancelParams, DirectionEnum, InsertParams, OrderTypeEnum},
    ws_client::WsClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Optionally set environment via env vars or modify WsClient::from_env for custom env
    let client = WsClient::from_env().await.unwrap();

    let order = client
        .rpc()
        .trading()
        .insert(InsertParams {
            direction: DirectionEnum::Buy,
            amount: rust_decimal_macros::dec!(0.0001),
            price: Some(rust_decimal_macros::dec!(50000.0)),
            instrument_name: Some("BTC-PERPETUAL".to_string()),
            order_type: Some(OrderTypeEnum::Limit),
            post_only: Some(true),
            reject_post_only: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();

    println!("Order placed: {:?}", order);

    // cancel the order
    client
        .rpc()
        .trading()
        .cancel(CancelParams {
            order_id: Some(order.order_id.clone()),
            ..Default::default()
        })
        .await
        .unwrap();

    Ok(())
}
