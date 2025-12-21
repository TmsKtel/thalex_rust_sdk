use crate::{
    models::{Notifications, UserInboxNotificationsNotification},
    ws_client::{RequestScope, WsClient},
};
type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct NotificationsSubscriptions<'a> {
    pub client: &'a WsClient,
}
impl<'a> NotificationsSubscriptions<'a> {
    pub async fn user_inbox_notifications<F>(&self, mut callback: F) -> Result<(), Error>
    where
        F: FnMut(Notifications) + Send + 'static,
    {
        let channel = "user.inbox_notifications".to_string();
        // Per-subscription channel from core -> user callback
        self.client
            .subscribe_channel(
                RequestScope::Private,
                channel,
                move |msg: UserInboxNotificationsNotification| {
                    callback(msg.notification);
                },
            )
            .await?;
        Ok(())
    }
}
