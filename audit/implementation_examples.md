# Полные примеры кода для внедрения оптимизаций

Этот документ содержит полные примеры кода "до" и "после" для каждой оптимизации.

---

## 1. Убрать лишние копирования входящих сообщений

### В run_single_connection

**До (строки 299, 302):**
```rust
Some(Ok(Message::Text(text))) => {
    handle_incoming(text.to_string(), pending_requests, subscriptions).await;  // Лишнее копирование
}
Some(Ok(Message::Binary(bin))) => {
    if let Ok(text) = String::from_utf8(bin.to_vec()) {  // Лишнее копирование буфера
        handle_incoming(text, pending_requests, subscriptions).await;
    }
}
```

**После:**
```rust
Some(Ok(Message::Text(text))) => {
    handle_incoming(text, pending_requests, subscriptions).await;  // text уже String
}
Some(Ok(Message::Binary(bin))) => {
    if let Ok(text) = String::from_utf8(bin) {  // Без .to_vec()
        handle_incoming(text, pending_requests, subscriptions).await;
    }
}
```

**Ожидаемый эффект:** Снижение аллокаций/копирований на каждом сообщении.

---

## 2. Оптимизация JSON парсинга - полный код

### До оптимизации

```rust
async fn handle_incoming(
    text: String,
    pending_requests: &Arc<Mutex<HashMap<u64, ResponseSender>>>,
    subscriptions: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
) {
    let parsed: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to parse incoming message as JSON: {e}; raw: {text}");
            return;
        }
    };

    // RPC response: has "id"
    if let Some(id) = parsed.get("id").and_then(|v| v.as_u64()) {
        let mut pending = pending_requests.lock().await;
        if let Some(tx) = pending.remove(&id) {
            let _ = tx.send(text);
        } else {
            warn!("Received RPC response for unknown id={id}");
        }
        return;
    }

    // Subscription notification: has "channel_name"
    if let Some(channel_name) = parsed.get("channel_name").and_then(|v| v.as_str()) {
        let mut subs = subscriptions.lock().await;
        if let Some(sender) = subs.get_mut(channel_name) {
            if sender.send(text).is_err() {
                // Receiver dropped; cleanup this subscription entry.
                subs.remove(channel_name);
            }
        } else {
            warn!("Received message for unsubscribed channel: {channel_name}");
        }
        return;
    }

    warn!("Received unhandled message: {text}");
}
```

### После оптимизации

```rust
async fn handle_incoming(
    text: String,
    pending_requests: &Arc<Mutex<HashMap<u64, ResponseSender>>>,
    subscriptions: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
) {
    // Fast key checking before full parsing
    // ВАЖНО: В JSON-RPC поле "id" - это число, не строка. В JSON это "id":123, а не "id":"123"
    // Ищем "id": (для числового id) - без кавычек после двоеточия
    if text.contains("\"id\":") {
        // Парсим только если есть поле id
        // Используем Envelope parsing для подтверждения, что id действительно top-level
        match serde_json::from_str::<Value>(&text) {
            Ok(parsed) => {
                if let Some(id) = parsed.get("id").and_then(|v| v.as_u64()) {
                    // Remove под lock, send вне lock
                    let tx_opt = {
                        let mut pending = pending_requests.lock().await;
                        pending.remove(&id)
                    };
                    // Lock отпущен здесь
                    
                    if let Some(tx) = tx_opt {
                        let _ = tx.send(text);  // Отправка вне lock
                    } else {
                        warn!("Received RPC response for unknown id={id}");
                    }
                    return;
                }
            }
            Err(e) => {
                warn!("Failed to parse RPC response as JSON: {e}; raw: {text}");
                return;
            }
        }
    }

    // Check for subscription notification
    if text.contains("\"channel_name\":") {
        // Parse only if we have "channel_name" field
        match serde_json::from_str::<Value>(&text) {
            Ok(parsed) => {
                if let Some(channel_name) = parsed.get("channel_name").and_then(|v| v.as_str()) {
                    // Клонируем sender под lock, отправляем вне lock
                    let sender_opt = {
                        let mut subs = subscriptions.lock().await;
                        subs.get_mut(channel_name).map(|s| s.clone())  // UnboundedSender клонируется дёшево
                    };
                    // Lock отпущен здесь
                    
                    if let Some(mut sender) = sender_opt {
                        if sender.send(text).is_err() {
                            // Если send failed, коротко взять lock и удалить entry
                            let mut subs = subscriptions.lock().await;
                            subs.remove(channel_name);
                        }
                    } else {
                        warn!("Received message for unsubscribed channel: {channel_name}");
                    }
                    return;
                }
            }
            Err(e) => {
                warn!("Failed to parse subscription message as JSON: {e}; raw: {text}");
                return;
            }
        }
    }

    // If neither key found, log as unhandled
    warn!("Received unhandled message (no 'id' or 'channel_name'): {text}");
}
```

**Изменения:**
1. Добавлена быстрая проверка `contains()` перед парсингом (ищем `"id":` для числового id)
2. Парсинг выполняется только если нужный ключ найден
3. **Отправка вне lock** - уменьшает contention
4. Сохранена обработка ошибок
5. Сохранена логика обработки всех случаев

**Важное замечание:** Проверка `contains("\"id\":")` может найти `"id":` во вложенных объектах. Для полной надежности рекомендуется использовать Envelope parsing (см. ниже).

---

## 2. DashMap для subscriptions - полная миграция

### Шаг 1: Добавить зависимость в Cargo.toml

```toml
[dependencies]
dashmap = "5.5"
# ... остальные зависимости
```

### Шаг 2: Изменить структуру WsClient

**До:**
```rust
pub struct WsClient {
    write_tx: mpsc::UnboundedSender<InternalCommand>,
    pending_requests: Arc<Mutex<HashMap<u64, ResponseSender>>>,
    subscriptions: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
    next_id: Arc<AtomicU64>,
    shutdown_tx: watch::Sender<bool>,
}
```

**После:**
```rust
use dashmap::DashMap;

pub struct WsClient {
    write_tx: mpsc::UnboundedSender<InternalCommand>,
    pending_requests: Arc<Mutex<HashMap<u64, ResponseSender>>>,
    subscriptions: Arc<DashMap<String, mpsc::UnboundedSender<String>>>,
    next_id: Arc<AtomicU64>,
    shutdown_tx: watch::Sender<bool>,
}
```

### Шаг 3: Изменить инициализацию в connect()

**До:**
```rust
let subscriptions = Arc::new(Mutex::new(HashMap::new()));
```

**После:**
```rust
let subscriptions = Arc::new(DashMap::new());
```

### Шаг 4: Изменить subscribe()

**До:**
```rust
{
    let mut subs = self.subscriptions.lock().await;
    subs.insert(channel.clone(), tx);
}
```

**После:**
```rust
self.subscriptions.insert(channel.clone(), tx);
```

### Шаг 5: Изменить unsubscribe()

**До:**
```rust
{
    let mut subs = self.subscriptions.lock().await;
    subs.remove(&channel);
}
```

**После:**
```rust
self.subscriptions.remove(&channel);
```

### Шаг 6: Изменить handle_incoming()

**До:**
```rust
if let Some(channel_name) = parsed.get("channel_name").and_then(|v| v.as_str()) {
    // Clone sender under lock, send outside lock
    let sender_opt = {
        let mut subs = subscriptions.lock().await;
        subs.get_mut(channel_name).map(|s| s.clone())  // UnboundedSender клонируется дёшево
    };
    // Lock отпущен здесь
    
    if let Some(mut sender) = sender_opt {
        if sender.send(text).is_err() {
            let mut subs = subscriptions.lock().await;
            subs.remove(channel_name);
        }
    }
}
```

**После (с DashMap, lock не нужен):**
```rust
if let Some(channel_name) = parsed.get("channel_name").and_then(|v| v.as_str()) {
    if let Some(mut sender) = subscriptions.get_mut(channel_name) {
        if sender.send(text).is_err() {
            // Receiver dropped; cleanup this subscription entry.
            subscriptions.remove(channel_name);
        }
    } else {
        warn!("Received message for unsubscribed channel: {channel_name}");
    }
}
```

### Шаг 7: Изменить run_single_connection() для переподписки

**До:**
```rust
{
    let subs = subscriptions.lock().await;
    for channel in subs.keys() {
        let msg = serde_json::json!({
            "method": "public/subscribe",
            "params": { "channels": [channel] },
        });
        ws.send(Message::Text(msg.to_string().into())).await?;
    }
}
```

**После:**
```rust
// Batch all subscriptions in one message
let channels: Vec<String> = subscriptions.iter().map(|entry| entry.key().clone()).collect();
if !channels.is_empty() {
    let msg = serde_json::json!({
        "method": "public/subscribe",
        "params": { "channels": channels },
    });
    ws.send(Message::Text(msg.to_string().into())).await?;
}
```

**Важно:** DashMap не требует блокировки для чтения, но `get_mut()` возвращает `RefMut`, который нужно использовать аккуратно.

---

## 5. Экспоненциальный backoff - полный код

### Добавить зависимость (опционально)

Если используем `fastrand`:
```toml
[dependencies]
fastrand = "2.0"
```

Или использовать стандартный `rand`:
```toml
[dependencies]
rand = "0.8"
```

### Полный код connection_supervisor с backoff

**До:**
```rust
async fn connection_supervisor(
    url: String,
    mut cmd_rx: mpsc::UnboundedReceiver<InternalCommand>,
    mut shutdown_rx: watch::Receiver<bool>,
    pending_requests: Arc<Mutex<HashMap<u64, ResponseSender>>>,
    subscriptions: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
) {
    info!("Connection supervisor started for {url}");

    loop {
        if *shutdown_rx.borrow() {
            info!("Supervisor sees shutdown for {url}");
            break;
        }

        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                // ... обработка соединения ...
                
                if *shutdown_rx.borrow() {
                    break;
                }

                info!("Reconnecting to {url} after backoff");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
            Err(e) => {
                error!("Failed to connect to {url}: {e}");
                if *shutdown_rx.borrow() || cmd_rx.is_closed() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }
    }
}
```

**После:**
```rust
async fn connection_supervisor(
    url: String,
    mut cmd_rx: mpsc::UnboundedReceiver<InternalCommand>,
    mut shutdown_rx: watch::Receiver<bool>,
    pending_requests: Arc<Mutex<HashMap<u64, ResponseSender>>>,
    subscriptions: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
) {
    info!("Connection supervisor started for {url}");

    let mut backoff_secs = 1u64;
    const MAX_BACKOFF: u64 = 60;
    const INITIAL_BACKOFF: u64 = 1;

    loop {
        if *shutdown_rx.borrow() {
            info!("Supervisor sees shutdown for {url}");
            break;
        }

        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                info!("WebSocket connected to {url}");
                
                // Reset backoff on successful connection
                backoff_secs = INITIAL_BACKOFF;

                let result = run_single_connection(
                    &url,
                    ws_stream,
                    &mut cmd_rx,
                    &mut shutdown_rx,
                    &pending_requests,
                    &subscriptions,
                )
                .await;

                if let Err(e) = result {
                    error!("Connection error on {url}: {e}");
                }

                // Fail all pending RPCs on this connection.
                let mut pending = pending_requests.lock().await;
                for (_, tx) in pending.drain() {
                    let _ = tx.send(r#"{"error":"connection closed"}"#.to_string());
                }

                if *shutdown_rx.borrow() {
                    info!("Shutdown after connection end for {url}");
                    break;
                }

                if cmd_rx.is_closed() {
                    info!("Command channel closed for {url}, stopping supervisor");
                    break;
                }

                // Exponential backoff with jitter
                let jitter = fastrand::u64(0..=backoff_secs);
                let delay_secs = (backoff_secs + jitter).min(MAX_BACKOFF);
                info!("Reconnecting to {url} after {delay_secs}s backoff");
                tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF);
            }
            Err(e) => {
                error!("Failed to connect to {url}: {e}");
                if *shutdown_rx.borrow() || cmd_rx.is_closed() {
                    break;
                }
                
                // Exponential backoff with jitter
                let jitter = fastrand::u64(0..=backoff_secs);
                let delay_secs = (backoff_secs + jitter).min(MAX_BACKOFF);
                info!("Reconnecting to {url} after {delay_secs}s backoff");
                tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF);
            }
        }
    }

    info!("Connection supervisor exited for {url}");
}
```

**Альтернатива без fastrand (используя только стандартную библиотеку):**

```rust
// Вместо fastrand::u64 можно использовать простой счетчик
use std::time::{SystemTime, UNIX_EPOCH};

// Получить jitter из системного времени
let jitter = (SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_nanos() % (backoff_secs as u128 + 1)) as u64;
```

---

## 6. Батчинг переподписок - полный код

### Полный код run_single_connection с батчингом

**До (строки 256-266):**
```rust
// Re-subscribe active channels on new connection.
{
    let subs = subscriptions.lock().await;
    for channel in subs.keys() {
        let msg = serde_json::json!({
            "method": "public/subscribe",
            "params": { "channels": [channel] },
        });
        ws.send(Message::Text(msg.to_string().into())).await?;
    }
}
```

**После (с snapshot, без lock во время I/O):**
```rust
// Re-subscribe active channels on new connection (batched).
// Сначала делаем snapshot под lock
let channels: Vec<String> = {
    let subs = subscriptions.lock().await;
    subs.keys().map(|k| k.clone()).collect()  // Snapshot ключей
};
// Lock отпущен здесь

// Теперь отправляем вне lock
if !channels.is_empty() {
    let msg = serde_json::json!({
        "method": "public/subscribe",
        "params": { "channels": channels },
    });
    ws.send(Message::Text(msg.to_string().into())).await?;
    info!("Re-subscribed to {} channels", channels.len());
}
```

**Если используется DashMap:**
```rust
// Re-subscribe active channels on new connection (batched).
let channels: Vec<String> = subscriptions.iter().map(|entry| entry.key().clone()).collect();
if !channels.is_empty() {
    let msg = serde_json::json!({
        "method": "public/subscribe",
        "params": { "channels": channels },
    });
    ws.send(Message::Text(msg.to_string().into())).await?;
    info!("Re-subscribed to {} channels", channels.len());
}
```

---

## 7. Уменьшение клонирования строк - конкретные примеры

### Пример 1: subscribe() - убрать лишнее клонирование

**До (строка 115, 122):**
```rust
pub async fn subscribe<F>(&self, channel: &str, mut callback: F) -> Result<(), Error>
where
    F: FnMut(String) + Send + 'static,
{
    let channel = channel.to_string(); // Клонирование 1

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    {
        let mut subs = self.subscriptions.lock().await;
        subs.insert(channel.clone(), tx); // Клонирование 2
    }

    let msg = serde_json::json!({
        "method": "public/subscribe",
        "params": {
            "channels": [channel] // Используем уже созданную строку
        }
    });
    // ...
}
```

**После:**
```rust
pub async fn subscribe<F>(&self, channel: &str, mut callback: F) -> Result<(), Error>
where
    F: FnMut(String) + Send + 'static,
{
    // Клонируем только один раз, когда действительно нужно владение
    let channel = channel.to_string();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    {
        let mut subs = self.subscriptions.lock().await;
        // Используем уже созданную строку, не клонируем снова
        subs.insert(channel.clone(), tx);
    }

    let msg = serde_json::json!({
        "method": "public/subscribe",
        "params": {
            "channels": [channel] // Используем уже созданную строку
        }
    });
    // ...
}
```

**Еще лучше - использовать channel напрямую:**
```rust
pub async fn subscribe<F>(&self, channel: &str, mut callback: F) -> Result<(), Error>
where
    F: FnMut(String) + Send + 'static,
{
    let channel_owned = channel.to_string();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    {
        let mut subs = self.subscriptions.lock().await;
        // Вставляем channel_owned, больше не нужно
        subs.insert(channel_owned.clone(), tx);
    }

    let msg = serde_json::json!({
        "method": "public/subscribe",
        "params": {
            "channels": [channel_owned] // Используем уже созданную строку
        }
    });
    // ...
}
```

### Пример 2: handle_incoming - принимать по ссылке

**До:**
```rust
async fn handle_incoming(
    text: String, // Принимается по значению
    // ...
) {
    // text используется, но не владеет им после отправки
    let _ = tx.send(text); // Передача владения
}
```

**После:**
```rust
async fn handle_incoming(
    text: &str, // Принимается по ссылке
    pending_requests: &Arc<Mutex<HashMap<u64, ResponseSender>>>,
    subscriptions: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
) {
    // Клонируем только когда нужно отправить
    if text.contains("\"id\":") {
        match serde_json::from_str::<Value>(text) {
            Ok(parsed) => {
                if let Some(id) = parsed.get("id").and_then(|v| v.as_u64()) {
                    let mut pending = pending_requests.lock().await;
                    if let Some(tx) = pending.remove(&id) {
                        // Клонируем только здесь, когда нужно отправить
                        let _ = tx.send(text.to_string());
                    }
                    return;
                }
            }
            // ...
        }
    }
    // ...
}
```

**Но нужно изменить вызов:**
```rust
// В run_single_connection
Some(Ok(Message::Text(text))) => {
    handle_incoming(&text, pending_requests, subscriptions).await;
}
```

---

## 8. Полный список зависимостей для всех оптимизаций

```toml
[dependencies]
# Существующие
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.28", features = ["native-tls"] }
futures-util = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = "0.4.29"
simple_logger = "5.1.0"

# Новые для оптимизаций
dashmap = "5.5"  # Для concurrent HashMap
fastrand = "2.0"  # Для jitter в backoff (опционально, можно использовать rand или SystemTime)
```

---

## 9. Чеклист проверки после внедрения

### После оптимизации JSON парсинга:
- [ ] Сообщения с "id" обрабатываются корректно
- [ ] Сообщения с "channel_name" обрабатываются корректно
- [ ] Нераспознанные сообщения логируются
- [ ] Ошибки парсинга обрабатываются
- [ ] Бенчмарки показывают улучшение

### После миграции на DashMap:
- [ ] Подписки работают корректно
- [ ] Отписки работают корректно
- [ ] Переподписка при переподключении работает
- [ ] Нет deadlock'ов
- [ ] Бенчмарки показывают улучшение конкурентной обработки

### После батчинга переподписок:
- [ ] Переподписка работает быстрее
- [ ] API принимает батчинг (проверить документацию)
- [ ] Все каналы восстанавливаются корректно

### После экспоненциального backoff:
- [ ] Backoff увеличивается при ошибках
- [ ] Backoff сбрасывается при успешном подключении
- [ ] Jitter работает (разные задержки)
- [ ] Максимальный backoff не превышается

---

## 10. Возможные проблемы и решения

### Проблема 1: DashMap API отличается от HashMap

**Симптом:** Ошибки компиляции при замене.

**Решение:**
- `get_mut()` возвращает `RefMut`, а не `&mut`
- Использовать `if let Some(mut entry) = map.get_mut(key)` вместо `if let Some(value) = map.get_mut(key)`
- Для итерации использовать `map.iter()` вместо `map.keys()`

### Проблема 2: Lifetime issues при изменении handle_incoming на &str

**Симптом:** Ошибки компиляции о lifetime.

**Решение:**
- Убедиться, что `text` живет достаточно долго
- Возможно, нужно оставить `String` и клонировать только при отправке

### Проблема 3: API не поддерживает батчинг подписок

**Симптом:** Сервер возвращает ошибку при батчинге.

**Решение:**
- Проверить документацию API
- Если не поддерживается, оставить отдельные сообщения
- Или отправить батч, но обработать ошибку и отправить по одному

### Проблема 4: fastrand не доступен

**Симптом:** Ошибка компиляции.

**Решение:**
- Использовать альтернативу с `SystemTime` (см. пример выше)
- Или использовать `rand` crate
- Или использовать простой счетчик для детерминированного jitter

---

## 11. Примеры тестов

### Тест для оптимизации JSON парсинга

```rust
#[tokio::test]
async fn test_handle_incoming_rpc_fast_path() {
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let subs = Arc::new(Mutex::new(HashMap::new()));
    
    let (tx, rx) = oneshot::channel();
    pending.lock().await.insert(123, tx);
    
    let msg = r#"{"jsonrpc":"2.0","id":123,"result":{"status":"ok"}}"#;
    handle_incoming(msg.to_string(), &pending, &subs).await;
    
    let response = rx.await.unwrap();
    assert_eq!(response, msg);
}

#[tokio::test]
async fn test_handle_incoming_subscription_fast_path() {
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let subs = Arc::new(Mutex::new(HashMap::new()));
    
    let (tx, mut rx) = mpsc::unbounded_channel();
    subs.lock().await.insert("test.channel".to_string(), tx);
    
    let msg = r#"{"channel_name":"test.channel","notification":{"data":"test"}}"#;
    handle_incoming(msg.to_string(), &pending, &subs).await;
    
    let received = rx.recv().await.unwrap();
    assert_eq!(received, msg);
}
```

### Тест для DashMap

```rust
#[tokio::test]
async fn test_dashmap_concurrent_access() {
    use dashmap::DashMap;
    use std::sync::Arc;
    
    let map: Arc<DashMap<String, u64>> = Arc::new(DashMap::new());
    
    // Concurrent writes
    let mut handles = Vec::new();
    for i in 0..10 {
        let map_clone = map.clone();
        let handle = tokio::spawn(async move {
            map_clone.insert(format!("key_{}", i), i);
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
    
    assert_eq!(map.len(), 10);
}
```

---

## Заключение

Эти примеры должны помочь заказчику внедрить оптимизации. Для сложных оптимизаций рекомендуется:

1. Внедрять по одной оптимизации
2. Тестировать после каждой
3. Запускать бенчмарки для проверки улучшений
4. Откатывать изменения, если что-то не работает

