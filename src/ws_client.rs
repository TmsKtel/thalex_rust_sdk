use tokio::net::TcpStream;

use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::protocol::Message,
};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsWrite = SplitSink<WsStream, Message>;
type WsRead = SplitStream<WsStream>;
type Callback = Box<dyn FnMut(String) + Send + 'static>;

pub struct WsClient {
    url: String,
    write: Arc<Mutex<Option<WsWrite>>>,
    read: Arc<Mutex<Option<WsRead>>>,
    callbacks: Arc<Mutex<HashMap<String, Callback>>>,
}
const URL: &str = "wss://thalex.com/ws/api/v2";

impl Default for WsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl WsClient {
    pub fn new() -> Self {
        WsClient {
            url: URL.into(),
            write: Arc::new(Mutex::new(None)),
            read: Arc::new(Mutex::new(None)),
            callbacks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        println!("WebSocket connected to {}", &self.url);

        let (write, read) = ws_stream.split();
        *self.write.lock().await = Some(write);
        *self.read.lock().await = Some(read);

        Ok(())
    }

    pub async fn subscribe<F>(
        &self,
        channel: &str,
        callback: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(String) + Send + 'static,
    {
        let mut write_guard = self.write.lock().await;
        let write = write_guard
            .as_mut()
            .ok_or("Not connected - call connect() first")?;

        let subscribe_msg = serde_json::json!({
            "method": "public/subscribe",
            "params": {
                "channels": [channel]
            }
        });

        write
            .send(Message::Text(subscribe_msg.to_string().into()))
            .await?;

        // Store callback for this channel
        self.callbacks
            .lock()
            .await
            .insert(channel.to_string(), Box::new(callback));
        println!("Subscribed to channel: {channel}");

        Ok(())
    }

    pub async fn unsubscribe(&self, channel: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut write_guard = self.write.lock().await;
        let write = write_guard
            .as_mut()
            .ok_or("Not connected - call connect() first")?;

        let unsubscribe_msg = serde_json::json!({
            "method": "unsubscribe",
            "params": {
                "channels": [channel]
            }
        });

        write
            .send(Message::Text(unsubscribe_msg.to_string().into()))
            .await?;

        // Remove callback for this channel
        self.callbacks.lock().await.remove(channel);
        println!("Unsubscribed from channel: {channel}");

        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut write_guard = self.write.lock().await;
        if let Some(write) = write_guard.as_mut() {
            write.send(Message::Close(None)).await?;
            println!("WebSocket disconnected from {}", &self.url);
        }

        *write_guard = None;
        *self.read.lock().await = None;
        self.callbacks.lock().await.clear();

        Ok(())
    }

    pub async fn run_forever(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut read_guard = self.read.lock().await;
        let read = read_guard
            .as_mut()
            .ok_or("Not connected - call connect() first")?;

        while let Some(message) = read.next().await {
            match message? {
                Message::Text(text) => {
                    self.handle_message(text.to_string()).await;
                }
                Message::Binary(bin) => {
                    if let Ok(text) = String::from_utf8(bin.into()) {
                        println!("Received Binary Message: {}", text.clone());
                        self.handle_message(text).await;
                    }
                }
                Message::Ping(data) => {
                    let mut write = self.write.lock().await;
                    if let Some(w) = write.as_mut() {
                        w.send(Message::Pong(data)).await?;
                    }
                }
                Message::Close(_) => {
                    println!("WebSocket connection closed");
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_message(&self, text: String) {
        // Try to extract channel name from message
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(channel_name) = parsed.get("channel_name").and_then(|v| v.as_str()) {
                // Call the specific callback for this channel
                let mut callbacks = self.callbacks.lock().await;
                if let Some(callback) = callbacks.get_mut(channel_name) {
                    // we should really do something clever here with the bytes... :)
                    let string_data = text.to_string();
                    callback(string_data);
                }
            }
        }
    }

    pub async fn send_raw(&self, msg: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut write_guard = self.write.lock().await;
        let write = write_guard
            .as_mut()
            .ok_or("Not connected - call connect() first")?;

        write.send(Message::Text(msg.to_string().into())).await?;
        Ok(())
    }
}
