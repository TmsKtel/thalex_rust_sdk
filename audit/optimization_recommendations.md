# Рекомендации по оптимизации

## 1. Убрать лишние копирования входящих сообщений (самый дешёвый выигрыш)

### Проблема
В `run_single_connection` (строки 299, 302) происходят лишние аллокации:
- `Message::Text(text)` → `handle_incoming(text.to_string(), ...)` - `text` уже `String`
- `Message::Binary(bin)` → `String::from_utf8(bin.to_vec())` - лишнее копирование буфера

### Решение
```rust
// Строка 299: убрать .to_string()
Some(Ok(Message::Text(text))) => {
    handle_incoming(text, pending_requests, subscriptions).await;  // text уже String
}

// Строка 302: убрать .to_vec()
Some(Ok(Message::Binary(bin))) => {
    if let Ok(text) = String::from_utf8(bin) {  // без .to_vec()
        handle_incoming(text, pending_requests, subscriptions).await;
    }
}
```

**Ожидаемый эффект:** Снижение аллокаций/копирований на каждом сообщении.

---

## 2. Оптимизация блокировок

### Проблема
Mutex блокировки в `handle_incoming` создают узкое место при высокой частоте сообщений. Кроме того, lock удерживается во время `send()`, что увеличивает contention.

### Решение A: Отправка вне lock (быстрая оптимизация)

**Для subscriptions (строка 360-365):**
```rust
// Сейчас: lock удерживается во время send
let mut subs = subscriptions.lock().await;
if let Some(sender) = subs.get_mut(channel_name) {
    if sender.send(text).is_err() {
        subs.remove(channel_name);
    }
}

// После оптимизации: клонируем sender под lock, отпускаем lock, отправляем вне lock
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
}
```

**Для pending_requests (строка 349-351):**
```rust
// Сейчас: lock удерживается во время send
let mut pending = pending_requests.lock().await;
if let Some(tx) = pending.remove(&id) {
    let _ = tx.send(text);
}

// После оптимизации: remove под lock, send вне lock
let tx_opt = {
    let mut pending = pending_requests.lock().await;
    pending.remove(&id)
};
// Lock отпущен здесь

if let Some(tx) = tx_opt {
    let _ = tx.send(text);  // Отправка вне lock
}
```

**Преимущества:**
- Снижает contention между обработкой входящих сообщений и `subscribe/unsubscribe/call_rpc`
- Уменьшает время удержания lock
- Безопасно (UnboundedSender клонируется дёшево)

### Решение B: Использование DashMap для concurrent access

Заменить `Arc<Mutex<HashMap>>` на `Arc<DashMap>` для lock-free чтения:

```rust
use dashmap::DashMap;

pub struct WsClient {
    // ...
    pending_requests: Arc<DashMap<u64, ResponseSender>>,
    subscriptions: Arc<DashMap<String, mpsc::UnboundedSender<String>>>,
    // ...
}
```

**Преимущества:**
- Lock-free чтение
- Параллельный доступ к разным ключам
- Меньше contention

**Недостатки:**
- Дополнительная зависимость
- Немного больше памяти

### Решение B: Разделение на читающие и пишущие блокировки

Использовать `RwLock` вместо `Mutex`:

```rust
use tokio::sync::RwLock;

pending_requests: Arc<RwLock<HashMap<u64, ResponseSender>>>,
```

**Преимущества:**
- Множественные читатели одновременно
- Меньше contention для read-heavy workload

**Недостатки:**
- Writer может блокировать всех читателей
- RwLock может быть медленнее Mutex при высокой конкуренции

### Рекомендация
Сначала применить "отправку вне lock" (Решение A) - это быстрая и безопасная оптимизация. Затем можно рассмотреть DashMap для `subscriptions` (read-heavy) и оставить Mutex для `pending_requests` (write-heavy, но короткоживущие).

## 3. Оптимизация JSON парсинга

### Проблема
Полный парсинг каждого сообщения даже когда нужны только определенные поля.

### Решение A: Быстрая проверка + полный парсинг (простой вариант)

**Важно:** В JSON-RPC поле `id` - это число, а не строка. В JSON это будет `"id":123`, а не `"id":"123"`.

```rust
async fn handle_incoming(
    text: String,
    pending_requests: &Arc<DashMap<u64, ResponseSender>>,
    subscriptions: &Arc<DashMap<String, mpsc::UnboundedSender<String>>>,
) {
    // Быстрая проверка без полного парсинга
    // Ищем "id": (для числового id) или "id": (может быть строка в некоторых случаях)
    if text.contains("\"id\":") || text.find("\"id\":").is_some() {
        // Парсим только если есть поле id
        if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
            if let Some(id) = parsed.get("id").and_then(|v| v.as_u64()) {
                // обработка RPC ответа
                if let Some((_, tx)) = pending_requests.remove(&id) {
                    let _ = tx.send(text);
                }
                return;
            }
        }
    }
    
    // Аналогично для channel_name
    if text.contains("\"channel_name\":") {
        // ...
    }
}
```

**Проблема:** `contains()` может найти `"id":` во вложенных объектах. Нужна двухэтапная проверка.

### Решение B: Envelope parsing (рекомендуется)

Использовать легкий struct для парсинга только нужных полей вместо полного `Value`:

```rust
#[derive(Deserialize)]
struct Envelope<'a> {
    id: Option<u64>,
    #[serde(borrow)]
    channel_name: Option<std::borrow::Cow<'a, str>>,
}

async fn handle_incoming(
    text: &str,  // Принимаем по ссылке
    pending_requests: &Arc<DashMap<u64, ResponseSender>>,
    subscriptions: &Arc<DashMap<String, mpsc::UnboundedSender<String>>>,
) {
    // Быстрая проверка для отсева неинтересных сообщений
    if !text.contains("\"id\":") && !text.contains("\"channel_name\":") {
        return;
    }
    
    // Легкий парсинг только нужных полей
    match serde_json::from_str::<Envelope>(text) {
        Ok(envelope) => {
            if let Some(id) = envelope.id {
                // RPC path
                if let Some((_, tx)) = pending_requests.remove(&id) {
                    let _ = tx.send(text.to_string()); // Клонируем только здесь
                }
                return;
            }
            
            if let Some(channel_name) = envelope.channel_name {
                // Subscription path
                let channel_str = channel_name.as_ref();
                if let Some(mut sender) = subscriptions.get_mut(channel_str) {
                    if sender.send(text.to_string()).is_err() {
                        subscriptions.remove(channel_str);
                    }
                }
                return;
            }
        }
        Err(e) => {
            warn!("Failed to parse message envelope: {e}");
            return;
        }
    }
}
```

**Преимущества Envelope parsing:**
- Меньше аллокаций (не строит полное дерево Value)
- Меньше CPU на парсинг
- Меньше pressure на allocator
- Подтверждает, что `id` действительно top-level (избегает ложноположительных)

**Альтернатива:** Использовать streaming JSON parser или ручной парсинг только нужных полей.

### Рекомендация
Рекомендуется использовать Envelope parsing (Решение B) для лучшей производительности. Если нужна простота, можно начать с быстрой проверки + полного парсинга (Решение A), но обязательно исправить проверку `"id"` для числового значения.

## 4. Батчинг переподписок

### Проблема
Отдельное сообщение для каждого канала при переподключении.

### Решение
Отправить одну команду со всеми каналами. **Важно:** Не держать lock во время I/O.

```rust
// В run_single_connection, строки 257-266
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

**Преимущества:**
- Меньше сетевых round-trips
- Быстрее восстановление подписок
- Меньше сериализации JSON
- **Lock не удерживается во время I/O** - уменьшает contention

**Если используется DashMap:**
```rust
// DashMap не требует lock для чтения
let channels: Vec<String> = subscriptions.iter()
    .map(|entry| entry.key().clone())
    .collect();

if !channels.is_empty() {
    let msg = serde_json::json!({
        "method": "public/subscribe",
        "params": { "channels": channels },
    });
    ws.send(Message::Text(msg.to_string().into())).await?;
    info!("Re-subscribed to {} channels", channels.len());
}
```

## 5. Уменьшение клонирования строк

### Проблема
Избыточное создание строк в нескольких местах.

### Решения

#### A. Использовать `&str` где возможно
```rust
pub async fn subscribe<F>(&self, channel: &str, mut callback: F) -> Result<(), Error>
where
    F: FnMut(String) + Send + 'static,
{
    // Не клонировать channel до необходимости
    let channel = channel.to_string(); // Только если действительно нужно владение
    // ...
}
```

#### B. Передавать строки по ссылке в handle_incoming
```rust
async fn handle_incoming(
    text: &str, // Изменить на &str
    // ...
) {
    // Использовать text напрямую, клонировать только при необходимости
}
```

#### C. Использовать `Cow<str>` для условного владения
```rust
use std::borrow::Cow;

async fn handle_incoming(
    text: Cow<'_, str>,
    // ...
)
```

## 6. Экспоненциальный backoff для переподключения

### Проблема
Фиксированная задержка 3 секунды.

### Решение
```rust
async fn connection_supervisor(
    // ...
) {
    let mut backoff_secs = 1u64;
    const MAX_BACKOFF: u64 = 60;
    
    loop {
        // ...
        if let Err(e) = result {
            error!("Connection error on {url}: {e}");
            
            // Fail all pending RPCs
            // ...
            
            if *shutdown_rx.borrow() {
                break;
            }
            
            // Экспоненциальный backoff с jitter
            let jitter = fastrand::u64(0..=backoff_secs);
            tokio::time::sleep(std::time::Duration::from_secs(backoff_secs + jitter)).await;
            backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF);
        } else {
            // Сброс backoff при успешном подключении
            backoff_secs = 1;
        }
    }
}
```

**Преимущества:**
- Меньше нагрузки на сервер при проблемах
- Быстрее восстановление при временных проблемах
- Jitter предотвращает thundering herd

## 7. Оптимизация структуры данных для subscriptions

### Проблема
HashMap с String ключами требует хеширования.

### Решение
Если channel names имеют ограниченный набор паттернов, можно использовать более эффективную структуру:

```rust
// Если channel names известны заранее, можно использовать enum
#[derive(Hash, Eq, PartialEq, Clone)]
enum Channel {
    Ticker(String, Delay), // "ticker.BTC-PERPETUAL.100ms"
    // другие типы каналов
}
```

Или использовать interned strings для уменьшения аллокаций:

```rust
use string_cache::DefaultAtom as Atom;

subscriptions: Arc<DashMap<Atom, mpsc::UnboundedSender<String>>>,
```

## 8. Пул буферов (опционально)

### Проблема
Много аллокаций для строк сообщений.

### Решение
Использовать `bytes::Bytes` или пул буферов:

```rust
use bytes::Bytes;

// В handle_incoming принимать Bytes вместо String
async fn handle_incoming(
    text: Bytes,
    // ...
)
```

Или использовать `bumpalo` для arena allocation в критических путях.

## 9. Предварительное резервирование HashMap ёмкости

### Проблема
HashMap растет динамически, что может вызывать реаллокации при большом количестве подписок или pending запросов.

### Решение
```rust
// При инициализации
let pending_requests = Arc::new(Mutex::new(HashMap::with_capacity(1024)));  // Ожидаемая верхняя граница
let subscriptions = Arc::new(Mutex::new(HashMap::with_capacity(128)));  // Типичное количество подписок
```

**Дополнительно:** При `subscribe()` если канал уже существует, не перетирать sender молча. Можно:
- Вернуть Err/лог warning и заменить аккуратно
- Или снять старый sender и дать ему завершиться

**Ожидаемый эффект:** Меньше реаллокаций, лучше контроль ресурсов.

---

## 10. Метрики и профилирование

### Рекомендация
Добавить метрики для мониторинга производительности:

```rust
use std::time::Instant;

async fn handle_incoming(
    text: String,
    // ...
) {
    let start = Instant::now();
    
    // ... обработка ...
    
    let duration = start.elapsed();
    if duration.as_millis() > 10 {
        warn!("Slow message handling: {:?}", duration);
    }
}
```

Или использовать `tracing` для структурированного логирования и профилирования.

## Приоритизация внедрения (минимум риска → максимум профита)

1. **Немедленно (безопасно и быстро):**
   - Убрать лишние копирования (оптимизация 1) - самый дешёвый выигрыш
   - Батчинг переподписок с snapshot (оптимизация 4) - безопасно, уменьшает lock+await проблему

2. **Высокий приоритет (безопасно, заметный эффект):**
   - Отправка вне lock (оптимизация 2, решение A) - безопасно, обычно даёт заметный эффект при конкуренции
   - Оптимизация JSON парсинга с Envelope (оптимизация 3, решение B) - средний риск, но большой выигрыш

3. **Средний приоритет:**
   - DashMap для subscriptions (оптимизация 2, решение B) - устраняет узкое место, но требует тестирования
   - Экспоненциальный backoff (оптимизация 6) - больше про reliability, чем про nanoseconds
   - Предварительное резервирование HashMap (оптимизация 9) - низкий риск

4. **Низкий приоритет:**
   - Уменьшение клонирования строк (оптимизация 5) - требует осторожности с lifetime
   - Пул буферов (оптимизация 8) - только если профилирование покажет необходимость

## Тестирование оптимизаций

Рекомендуется:
1. Создать benchmark тесты для измерения производительности
2. Использовать `criterion` для бенчмарков
3. Тестировать под нагрузкой (высокая частота сообщений)
4. Измерять latency и throughput до и после оптимизаций

