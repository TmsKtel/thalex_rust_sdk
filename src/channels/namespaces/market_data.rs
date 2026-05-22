use crate::{
    models::{
        BasePrice, BasePriceNotification, Book, BookNotification, Delay, Index, IndexComponents,
        IndexComponentsNotification, InstrumentsNotification, InstrumentsPayload, Lwt,
        LwtNotification, PriceIndexNotification, RecentTrades, RecentTradesNotification,
        RfqsNotification, RfqsPayload, Ticker, TickerNotification, UnderlyingStatistics,
        UnderlyingStatisticsNotification,
    },
    types::{Error, RequestScope},
    ws_client::WsClient,
};

pub struct MarketDataSubscriptions<'a> {
    pub client: &'a WsClient,
}
impl<'a> MarketDataSubscriptions<'a> {
    pub async fn ticker<F>(
        &self,
        instrument: &str,
        delay: Delay,
        mut callback: F,
    ) -> Result<String, Error>
    where
        F: FnMut(Ticker) + Send + 'static,
    {
        let channel = format!("ticker.{instrument}.{delay}");
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel.clone(),
                move |msg: TickerNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }

    pub async fn book<F>(
        &self,
        instrument: &str,
        grouping: &str,
        nlevels: &str,
        delay: Delay,
        mut callback: F,
    ) -> Result<String, Error>
    where
        F: FnMut(Book) + Send + 'static,
    {
        let channel = format!("book.{instrument}.{grouping}.{nlevels}.{delay}");
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel.clone(),
                move |msg: BookNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }

    pub async fn lwt<F>(
        &self,
        instrument: &str,
        delay: Delay,
        mut callback: F,
    ) -> Result<String, Error>
    where
        F: FnMut(Lwt) + Send + 'static,
    {
        let channel = format!("lwt.{instrument}.{delay}");
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel.clone(),
                move |msg: LwtNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }

    pub async fn recent_trades<F>(
        &self,
        target: &str,
        category: &str,
        mut callback: F,
    ) -> Result<String, Error>
    where
        F: FnMut(RecentTrades) + Send + 'static,
    {
        let channel = format!("recent_trades.{target}.{category}");
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel.clone(),
                move |msg: RecentTradesNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }

    pub async fn price_index<F>(&self, underlying: &str, mut callback: F) -> Result<String, Error>
    where
        F: FnMut(Index) + Send + 'static,
    {
        let channel = format!("price_index.{underlying}");
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel.clone(),
                move |msg: PriceIndexNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }

    pub async fn underlying_statistics<F>(
        &self,
        underlying: &str,
        mut callback: F,
    ) -> Result<String, Error>
    where
        F: FnMut(UnderlyingStatistics) + Send + 'static,
    {
        let channel = format!("underlying_statistics.{underlying}");
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel.clone(),
                move |msg: UnderlyingStatisticsNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }

    pub async fn base_price<F>(
        &self,
        underlying: &str,
        expiration: &str,
        mut callback: F,
    ) -> Result<String, Error>
    where
        F: FnMut(BasePrice) + Send + 'static,
    {
        let channel = format!("base_price.{underlying}.{expiration}");
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel.clone(),
                move |msg: BasePriceNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }

    pub async fn instruments<F>(&self, mut callback: F) -> Result<String, Error>
    where
        F: FnMut(InstrumentsPayload) + Send + 'static,
    {
        let channel = "instruments".to_string();
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel.clone(),
                move |msg: InstrumentsNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }

    pub async fn rfqs<F>(&self, mut callback: F) -> Result<String, Error>
    where
        F: FnMut(RfqsPayload) + Send + 'static,
    {
        let channel = "rfqs".to_string();
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel.clone(),
                move |msg: RfqsNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }

    pub async fn index_components<F>(
        &self,
        underlying: &str,
        mut callback: F,
    ) -> Result<String, Error>
    where
        F: FnMut(IndexComponents) + Send + 'static,
    {
        let channel = format!("index_components.{underlying}");
        self.client
            .subscribe_channel(
                RequestScope::Public,
                channel.clone(),
                move |msg: IndexComponentsNotification| callback(msg.notification),
            )
            .await?;
        Ok(channel)
    }
}
