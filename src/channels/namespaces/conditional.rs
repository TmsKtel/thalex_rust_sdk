use crate::{
    models::{AccountConditionalOrdersNotification, AccountConditionalOrdersPayload},
    types::{Error, RequestScope},
    ws_client::WsClient,
};

pub struct ConditionalSubscriptions<'a> {
    pub client: &'a WsClient,
}
impl<'a> ConditionalSubscriptions<'a> {
    pub async fn account_conditional_orders<F>(&self, mut callback: F) -> Result<String, Error>
    where
        F: FnMut(AccountConditionalOrdersPayload) + Send + 'static,
    {
        let channel = "account.conditional_orders".to_string();
        self.client
            .subscribe_channel(
                RequestScope::Private,
                channel.clone(),
                move |msg: AccountConditionalOrdersNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }
}
