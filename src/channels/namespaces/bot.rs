use crate::{
    models::{AccountBotsNotification, AccountBotsPayload},
    types::{Error, RequestScope},
    ws_client::WsClient,
};

pub struct BotSubscriptions<'a> {
    pub client: &'a WsClient,
}
impl<'a> BotSubscriptions<'a> {
    pub async fn account_bots<F>(&self, mut callback: F) -> Result<String, Error>
    where
        F: FnMut(AccountBotsPayload) + Send + 'static,
    {
        let channel = "account.bots".to_string();
        self.client
            .subscribe_channel(
                RequestScope::Private,
                channel.clone(),
                move |msg: AccountBotsNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }
}
