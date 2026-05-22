use crate::{
    models::{BannersNotification, BannersPayload, SystemEvent, SystemNotification},
    types::{Error, RequestScope},
    ws_client::WsClient,
};

pub struct SystemSubscriptions<'a> {
    pub client: &'a WsClient,
}
impl<'a> SystemSubscriptions<'a> {
    pub async fn system<F>(&self, mut callback: F) -> Result<String, Error>
    where
        F: FnMut(SystemEvent) + Send + 'static,
    {
        let channel = "system".to_string();
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel.clone(),
                move |msg: SystemNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }

    pub async fn banners<F>(&self, mut callback: F) -> Result<String, Error>
    where
        F: FnMut(BannersPayload) + Send + 'static,
    {
        let channel = "banners".to_string();
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel.clone(),
                move |msg: BannersNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }
}
