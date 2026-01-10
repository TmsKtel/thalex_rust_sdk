use crate::{
    models::{
        Bot, BotsResponse, CancelAllBotsResponse, CancelBotParams, CancelBotResponse,
        CreateBotParams, CreateBotResponse, RpcErrorResponse,
    },
    ws_client::WsClient,
};
use serde_json::Value;

pub struct BotRpc<'a> {
    pub client: &'a WsClient,
}
impl<'a> BotRpc<'a> {
    /// Get bots
    /// returns: Vec<Bot>
    pub async fn bots(&self) -> Result<Vec<Bot>, RpcErrorResponse> {
        let result: BotsResponse = self
            .client
            .send_rpc(
                "private/bots",
                serde_json::to_value(()).expect("Failed to serialize params"),
            )
            .await
            .expect("Failed to send RPC request");
        match result {
            BotsResponse::BotsResult(res) => Ok(res.result),
            BotsResponse::RpcErrorResponse(err) => Err(err),
        }
    }

    /// Create a bot
    /// returns: Bot
    pub async fn create_bot(&self, params: CreateBotParams) -> Result<Bot, RpcErrorResponse> {
        let result: CreateBotResponse = self
            .client
            .send_rpc(
                "private/create_bot",
                serde_json::to_value(params).expect("Failed to serialize params"),
            )
            .await
            .expect("Failed to send RPC request");
        match result {
            CreateBotResponse::CreateBotResult(res) => Ok(res.result),
            CreateBotResponse::RpcErrorResponse(err) => Err(err),
        }
    }

    /// Cancel a bot
    /// returns: Value
    pub async fn cancel_bot(&self, params: CancelBotParams) -> Result<Value, RpcErrorResponse> {
        let result: CancelBotResponse = self
            .client
            .send_rpc(
                "private/cancel_bot",
                serde_json::to_value(params).expect("Failed to serialize params"),
            )
            .await
            .expect("Failed to send RPC request");
        match result {
            CancelBotResponse::CancelBotResult(res) => Ok(res.result),
            CancelBotResponse::RpcErrorResponse(err) => Err(err),
        }
    }

    /// Cancel all bots
    /// returns: Value
    pub async fn cancel_all_bots(&self) -> Result<Value, RpcErrorResponse> {
        let result: CancelAllBotsResponse = self
            .client
            .send_rpc(
                "private/cancel_all_bots",
                serde_json::to_value(()).expect("Failed to serialize params"),
            )
            .await
            .expect("Failed to send RPC request");
        match result {
            CancelAllBotsResponse::CancelAllBotsResult(res) => Ok(res.result),
            CancelAllBotsResponse::RpcErrorResponse(err) => Err(err),
        }
    }
}
