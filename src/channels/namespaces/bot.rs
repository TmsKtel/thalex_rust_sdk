use crate::{
    models::{AccountBotsNotification, AccountBotsPayload},
    ws_client::{RequestScope, WsClient},
};
type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct BotSubscriptions<'a> {
    pub client: &'a WsClient,
}
impl<'a> BotSubscriptions<'a> {
    pub async fn account_bots<F>(&self, mut callback: F) -> Result<(), Error>
    where
        F: FnMut(AccountBotsPayload) + Send + 'static,
    {
        let channel = "account.bots".to_string();
        // Per-subscription channel from core -> user callback
        self.client
            .subscribe_channel(
                RequestScope::Private,
                channel,
                move |msg: AccountBotsNotification| {
                    callback(msg.notification);
                },
            )
            .await?;
        Ok(())
    }
}
