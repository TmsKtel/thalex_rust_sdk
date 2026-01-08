# Пошаговое руководство по внедрению оптимизаций

**Важно:** Текущий код использует два subscription map'а: `public_subscriptions` и `private_subscriptions`.  
Любой пример в этом руководстве, который ссылается на единый `subscriptions` map, является иллюстративным и должен быть адаптирован путем выбора соответствующего map'а.

Этот документ содержит детальные пошаговые инструкции для внедрения каждой оптимизации.

---

## Общие рекомендации

1. **Внедряйте по одной оптимизации** - не пытайтесь внедрить все сразу
2. **Делайте коммиты после каждой оптимизации** - это упростит откат при проблемах
3. **Запускайте тесты после каждого шага** - `cargo test`
4. **Запускайте бенчмарки** - `cargo bench` для проверки улучшений
5. **Проверяйте работоспособность** - запускайте примеры и проверяйте логи

---

## Оптимизация 1: Убрать лишние копирования (Приоритет 1, самый дешёвый выигрыш)

### Шаг 1: Найти места лишних копирований

Открыть `src/ws_client.rs`, найти функцию `resubscribe_all()` (обработчик переподписок при реконнекте).

### Шаг 2: Убрать text.to_string()

**Найти (строка 592):**
```rust
Some(Ok(Message::Text(text))) => {
    handle_incoming(
        text.to_string(),
        pending_requests,
        public_subscriptions,
        private_subscriptions,
    ).await;
}
```

**Заменить на:**
```rust
Some(Ok(Message::Text(text))) => {
    handle_incoming(
        text,  // text уже String
        pending_requests,
        public_subscriptions,
        private_subscriptions,
    ).await;
}
```

### Шаг 3: Убрать bin.to_vec()

**Найти (строка 599):**
```rust
Some(Ok(Message::Binary(bin))) => {
    if let Ok(text) = String::from_utf8(bin.to_vec()) {
        handle_incoming(
            text,
            pending_requests,
            public_subscriptions,
            private_subscriptions,
        ).await;
    }
}
```

**Заменить на:**
```rust
Some(Ok(Message::Binary(bin))) => {
    if let Ok(text) = String::from_utf8(bin) {  // Без .to_vec()
        handle_incoming(
            text,
            pending_requests,
            public_subscriptions,
            private_subscriptions,
        ).await;
    }
}
```

### Шаг 4: Проверить

1. `cargo build` - компиляция
2. `cargo test` - тесты
3. Запустить пример и проверить, что всё работает

### Ожидаемый результат
- Снижение аллокаций/копирований на каждом сообщении
- В профиле часто видно сразу

---

## Оптимизация 2: Батчинг переподписок (Приоритет 2, но проще всего)

### Шаг 1: Найти код переподписки

Открыть `src/ws_client.rs`, найти функцию `resubscribe_all`, строки 382-413.

### Шаг 2: Заменить код

**Найти:**
```rust
// Пример для public_subscriptions (аналогично для private_subscriptions)
{
    let subs = self.public_subscriptions.lock().await;
    for channel in subs.keys() {
        let msg = serde_json::json!({
            "method": "public/subscribe",
            "params": { "channels": [channel] },
        });
        ws.send(Message::Text(msg.to_string().into())).await?;
    }
}
```

**Заменить на (с snapshot, без lock во время I/O):**
```rust
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

### Шаг 3: Проверить

1. Запустить `cargo build` - должна быть успешная компиляция
2. Запустить `cargo test` - все тесты должны проходить
3. Запустить пример `cargo run --example subscribe` - проверить, что переподписка работает

### Шаг 4: Проверить API

**Важно:** Убедиться, что API Thalex поддерживает батчинг подписок. Проверить документацию или протестировать.

Если API не поддерживает:
- Откатить изменения
- Или добавить fallback: попробовать батч, при ошибке отправить по одному

### Ожидаемый результат
- Быстрее восстановление подписок после переподключения
- Меньше сообщений в логах о переподписке

---

## Оптимизация 3: JSON парсинг + отправка вне lock (Приоритет 1)

### Шаг 1: Найти функцию handle_incoming

Открыть `src/ws_client.rs`, найти функцию `handle_incoming` (строка 334).

### Шаг 2: Создать backup

Скопировать текущую функцию в комментарий или отдельный файл для reference.

### Шаг 3: Заменить логику парсинга

**Найти начало функции:**
```rust
let parsed: Value = match serde_json::from_str(&text) {
    Ok(v) => v,
    Err(e) => {
        warn!("Failed to parse incoming message as JSON: {e}; raw: {text}");
        return;
    }
};
```

**Заменить на (с отправкой вне lock):**
```rust
// Fast key checking before full parsing
// ВАЖНО: В JSON-RPC поле "id" - это число, не строка. В JSON это "id":123, а не "id":"123"
// Ищем "id": (для числового id) - без кавычек после двоеточия
if text.contains("\"id\":") {
    // This is likely an RPC response - parse only if needed
    let parsed: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to parse RPC response as JSON: {e}; raw: {text}");
            return;
        }
    };
    
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

if text.contains("\"channel_name\":") {
    // This is likely a subscription notification - parse only if needed
    let parsed: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to parse subscription message as JSON: {e}; raw: {text}");
            return;
        }
    };
    
    if let Some(channel_name) = parsed.get("channel_name").and_then(|v| v.as_str()) {
        // Клонируем sender под lock, отправляем вне lock
        // Проверяем оба map'а: public_subscriptions и private_subscriptions
        let sender_opt = {
            for route in [&private_subscriptions, &public_subscriptions] {
                let mut subs = route.lock().await;
                if let Some(sender) = subs.get_mut(channel_name) {
                    return Some(sender.clone());  // UnboundedSender клонируется дёшево
                }
            }
            None
        };
        // Lock отпущен здесь
        
        if let Some(mut sender) = sender_opt {
            if sender.send(text).is_err() {
                // Если send failed, коротко взять lock и удалить entry
                for route in [&private_subscriptions, &public_subscriptions] {
                    let mut subs = route.lock().await;
                    if subs.remove(channel_name).is_some() {
                        break;
                    }
                }
            }
        } else {
            warn!("Received message for unsubscribed channel: {channel_name}");
        }
        return;
    }
}

// If neither key found, log as unhandled
warn!("Received unhandled message (no 'id' or 'channel_name'): {text}");
```

**Важное замечание:** Проверка `contains("\"id\":")` может найти `"id":` во вложенных объектах (ложноположительные). Для полной надежности рекомендуется использовать Envelope parsing (см. раздел "Дополнительные оптимизации").

### Шаг 4: Удалить старый код

Удалить старую логику обработки после парсинга (строки после парсинга, которые теперь в блоках выше).

### Шаг 5: Проверить

1. `cargo build` - компиляция
2. `cargo test` - тесты
3. `cargo bench --bench json_parsing` - проверить улучшение
4. Запустить пример и проверить обработку сообщений

### Ожидаемый результат
- Бенчмарки показывают улучшение на ~2 µs для ticker сообщений
- Логи показывают корректную обработку сообщений

---

## Оптимизация 4: DashMap для subscriptions (Приоритет 3, сложная)

### Шаг 1: Добавить зависимость

Открыть `Cargo.toml`, добавить:
```toml
[dependencies]
dashmap = "5.5"
```

Запустить `cargo build` для загрузки зависимости.

### Шаг 2: Добавить use statement

В начале `src/ws_client.rs`, после других use:
```rust
use dashmap::DashMap;
```

### Шаг 3: Изменить тип в структуре WsClient

**Найти (строка 32):**
```rust
subscriptions: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
```

**Заменить на:**
```rust
subscriptions: Arc<DashMap<String, mpsc::UnboundedSender<String>>>,
```

### Шаг 4: Изменить инициализацию в connect()

**Найти (строка 51):**
```rust
let subscriptions = Arc::new(Mutex::new(HashMap::new()));
```

**Заменить на:**
```rust
let subscriptions = Arc::new(DashMap::new());
```

### Шаг 5: Изменить subscribe()

**Найти:**
```rust
// Выберите нужную таблицу подписок: public_subscriptions или private_subscriptions
{
    let mut subs = self.public_subscriptions.lock().await; // или self.private_subscriptions
    subs.insert(channel.clone(), tx);
}
```

**Заменить на:**
```rust
self.public_subscriptions.insert(channel.clone(), tx);  // или self.private_subscriptions
```

### Шаг 6: Изменить unsubscribe()

**Найти:**
```rust
{
    let mut subs = self.public_subscriptions.lock().await; // или self.private_subscriptions
    subs.remove(&channel);
}
```

**Заменить на:**
```rust
self.public_subscriptions.remove(&channel);  // Или private_subscriptions, в зависимости от контекста
```

### Шаг 7: Изменить handle_incoming()

**Найти:**
```rust
if let Some(channel_name) = parsed.get("channel_name").and_then(|v| v.as_str()) {
    // Проверяем оба map'а: public_subscriptions и private_subscriptions
    let sender_opt = {
        for route in [&private_subscriptions, &public_subscriptions] {
            let mut subs = route.lock().await;
            if let Some(sender) = subs.get_mut(channel_name) {
                return Some(sender.clone());  // UnboundedSender клонируется дёшево
            }
        }
        None
    };
    // Lock отпущен здесь
    
    if let Some(mut sender) = sender_opt {
        if sender.send(text).is_err() {
            // Если send failed, коротко взять lock и удалить entry
            for route in [&private_subscriptions, &public_subscriptions] {
                let mut subs = route.lock().await;
                if subs.remove(channel_name).is_some() {
                    break;
                }
            }
        }
    } else {
        warn!("Received message for unsubscribed channel: {channel_name}");
    }
    return;
}
```

### Шаг 8: Изменить resubscribe_all() для переподписки

**Найти (в resubscribe_all()):**
```rust
{
    let subs = self.public_subscriptions.lock().await;
    for channel in subs.keys() {
        let msg = serde_json::json!({
            "method": "public/subscribe",
            "params": { "channels": [channel] },
        });
        ws.send(Message::Text(msg.to_string().into())).await?;
    }
}
```

**Заменить на:**
```rust
// Batch all subscriptions in one message
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

### Шаг 9: Проверить компиляцию

1. `cargo build` - должны быть ошибки компиляции, если что-то пропущено
2. Исправить все ошибки
3. `cargo test` - все тесты должны проходить

### Шаг 10: Тестирование

1. Запустить пример: `cargo run --example subscribe`
2. Проверить, что подписки работают
3. Проверить, что переподписка работает после переподключения
4. Запустить бенчмарки: `cargo bench --bench mutex_contention`
5. Запустить бенчмарки: `cargo bench --bench subscription_throughput`

### Ожидаемый результат
- Бенчмарки показывают улучшение конкурентной обработки на 50-80%
- Нет deadlock'ов или race conditions
- Все функциональные тесты проходят

### Возможные проблемы

**Проблема:** Ошибка компиляции "cannot borrow as mutable"

**Решение:** DashMap `get_mut()` возвращает `RefMut`, нужно использовать правильно:
```rust
if let Some(mut entry) = subscriptions.get_mut(channel_name) {
    // entry - это RefMut, можно использовать как &mut
    if entry.send(text).is_err() {
        drop(entry); // Освободить RefMut перед remove
        subscriptions.remove(channel_name);
    }
}
```

---

## Оптимизация 5: Экспоненциальный backoff (Приоритет 5)

### Шаг 1: Решить, использовать ли fastrand

**Вариант A:** Использовать fastrand (проще)
- Добавить `fastrand = "2.0"` в `Cargo.toml`

**Вариант B:** Использовать SystemTime (без зависимостей)
- Не требует дополнительных зависимостей

Рекомендуется вариант A для простоты.

### Шаг 2: Добавить зависимость (если выбрали fastrand)

В `Cargo.toml`:
```toml
[dependencies]
fastrand = "2.0"
```

### Шаг 3: Найти connection_supervisor

Открыть `src/ws_client.rs`, найти функцию `connection_supervisor` (строка 182).

### Шаг 4: Добавить переменные backoff

В начале функции, после `info!("Connection supervisor started for {url}");`:
```rust
let mut backoff_secs = 1u64;
const MAX_BACKOFF: u64 = 60;
```

### Шаг 5: Изменить обработку успешного подключения

**Найти:**
```rust
Ok((ws_stream, _)) => {
    info!("WebSocket connected to {url}");
    // ...
}
```

**Добавить после `info!`:**
```rust
// Reset backoff on successful connection
backoff_secs = 1;
```

### Шаг 6: Заменить фиксированную задержку после успешного подключения

**Найти (строка 232):**
```rust
info!("Reconnecting to {url} after backoff");
tokio::time::sleep(std::time::Duration::from_secs(3)).await;
```

**Заменить на:**
```rust
// Exponential backoff with jitter
let jitter = fastrand::u64(0..=backoff_secs);
let delay_secs = (backoff_secs + jitter).min(MAX_BACKOFF);
info!("Reconnecting to {url} after {delay_secs}s backoff");
tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF);
```

### Шаг 7: Заменить фиксированную задержку при ошибке подключения

**Найти (строка 239):**
```rust
tokio::time::sleep(std::time::Duration::from_secs(3)).await;
```

**Заменить на:**
```rust
// Exponential backoff with jitter
let jitter = fastrand::u64(0..=backoff_secs);
let delay_secs = (backoff_secs + jitter).min(MAX_BACKOFF);
info!("Reconnecting to {url} after {delay_secs}s backoff");
tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF);
```

### Шаг 8: Проверить

1. `cargo build`
2. `cargo test`
3. Запустить пример и симулировать переподключение (отключить сеть)
4. Проверить логи - должны видеть увеличивающиеся задержки

### Ожидаемый результат
- При ошибках подключения задержка увеличивается: 1s, 2s, 4s, 8s, ...
- При успешном подключении задержка сбрасывается до 1s
- В логах видны разные задержки из-за jitter

---

## Оптимизация 6: Уменьшение клонирования строк (Приоритет 6)

### Шаг 1: Анализ мест клонирования

Найти все места, где происходит `to_string()` или клонирование строк:
- `subscribe_channel()` - строка 309
- `handle_incoming()` - строка 643 (принимает String)
- `resubscribe_all()` - обработчик переподписок при реконнекте
- `connection_supervisor()` - строка 503

### Шаг 2: Оптимизировать subscribe_channel()

**Найти (строка 309):**
```rust
let channel = channel.to_string();
```

**Проверить:** Это клонирование необходимо, так как channel нужен для HashMap ключа.

**Оптимизация:** Убедиться, что дополнительное клонирование не нужно, если channel уже String.

### Шаг 3: Оптимизировать handle_incoming (опционально, сложно)

**Внимание:** Изменение сигнатуры может потребовать изменений в других местах.

**Текущая сигнатура:**
```rust
async fn handle_incoming(
    text: String,
    // ...
)
```

**Можно изменить на:**
```rust
async fn handle_incoming(
    text: &str,
    // ...
)
```

**Но нужно:**
1. Изменить все вызовы `handle_incoming`
2. Клонировать `text` только при отправке: `tx.send(text.to_string())`

**Рекомендация:** Оставить как есть, если изменения слишком сложные. Фокус на других оптимизациях.

### Шаг 4: Проверить

1. `cargo build`
2. `cargo test`
3. Проверить, что функциональность не изменилась

---

## Порядок внедрения (рекомендуемый)

### Фаза 1: Быстрые победы (1 день)
1. ✅ Убрать лишние копирования (15 минут) - самый дешёвый выигрыш
2. ✅ Батчинг переподписок с snapshot (30 минут) - безопасно, уменьшает lock+await
3. ✅ JSON парсинг + отправка вне lock (1-2 часа) - безопасно, заметный эффект
4. ✅ Экспоненциальный backoff (1 час)

**Проверка:** Запустить все тесты и бенчмарки

### Фаза 2: Критическая оптимизация (1-2 дня)
1. ✅ Envelope parsing вместо полного Value (2-3 часа) - средний риск, большой выигрыш
2. ✅ DashMap для subscriptions (4-6 часов)
3. ✅ Тестирование и валидация (2-4 часа)

**Проверка:** 
- Все тесты проходят
- Бенчмарки показывают улучшение
- Интеграционные тесты проходят

### Фаза 3: Дополнительные (опционально)
1. Уменьшение клонирования строк (2-4 часа)

---

## Чеклист после внедрения всех оптимизаций

- [ ] Все тесты проходят: `cargo test`
- [ ] Бенчмарки показывают улучшение: `cargo bench`
- [ ] Примеры работают: `cargo run --example subscribe`
- [ ] Нет ошибок компиляции: `cargo build --release`
- [ ] Нет предупреждений: `cargo clippy`
- [ ] Логи показывают корректную работу
- [ ] Переподключение работает корректно
- [ ] Подписки восстанавливаются после переподключения

---

## Откат изменений

Если что-то пошло не так:

1. **Git откат:**
   ```bash
   git log  # Найти коммит до оптимизации
   git revert <commit-hash>
   ```

2. **Или откатить конкретную оптимизацию:**
   - Открыть файл
   - Откатить изменения вручную
   - Использовать backup, созданный на шаге 2

3. **Проверить:**
   ```bash
   cargo test
   cargo run --example subscribe
   ```

---

## Получение помощи

Если возникли проблемы:

1. Проверить примеры в `implementation_examples.md`
2. Проверить документацию библиотек (DashMap, fastrand)
3. Запустить `cargo clippy` для подсказок
4. Проверить логи при запуске примеров

---

## Заключение

Следуя этим инструкциям, заказчик сможет внедрить все оптимизации. Рекомендуется:

1. Начинать с простых оптимизаций
2. Тестировать после каждого шага
3. Делать коммиты для возможности отката
4. Использовать бенчмарки для проверки улучшений

