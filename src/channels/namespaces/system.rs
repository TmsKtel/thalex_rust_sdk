
use crate::{
    models::{
        BannersNotification, BannersPayload, SystemEvent, SystemNotification,
    },
    ws_client::{RequestScope, WsClient},
};
type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct SystemSubscriptions<'a> {
    pub client: &'a WsClient,
}
impl<'a> SystemSubscriptions<'a> {
    pub async fn system<F>(&self, mut callback: F) -> Result<(), Error>
    where
        F: FnMut(SystemEvent) + Send + 'static,
    {
        let channel = "system".to_string();
        // Per-subscription channel from core -> user callback
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel,
                move |msg: SystemNotification| {
                    callback(msg.notification);
                },
            )
            .await?;
        Ok(())
    }

    pub async fn banners<F>(&self, mut callback: F) -> Result<(), Error>
    where
        F: FnMut(BannersPayload) + Send + 'static,
    {
        let channel = "banners".to_string();
        // Per-subscription channel from core -> user callback
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel,
                move |msg: BannersNotification| {
                    callback(msg.notification);
                },
            )
            .await?;
        Ok(())
    }
}
