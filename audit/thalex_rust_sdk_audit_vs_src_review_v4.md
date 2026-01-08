# Review: соответствие audit-отчётов обновлённому исходному коду (thalex_rust_sdk4)

**Архив:** `thalex_rust_sdk4.tgz`  
**Объём проверки:** сравнил текущий код в `./src` (в первую очередь `src/ws_client.rs`) с рекомендациями и утверждениями в `./audit/*`, а также учёл ваши прошлые ревью-файлы `audit/thalex_rust_sdk_audit_*`.

> Важно: я НЕ предполагаю, что изменения в коде делались “по аудиту”. Я оцениваю только фактическое совпадение: *насколько выводы audit всё ещё верны* и *что стало устаревшим / появилось нового*.

---

## Итог

- Основные тезисы audit про **hot-path contention из-за `Mutex<HashMap<...>>`** и **полный JSON-парсинг каждого входящего сообщения** по-прежнему **актуальны** для текущего `src/ws_client.rs`.
- В коде есть **самостоятельные улучшения**, которых раньше не было: например, `resubscribe_all()` теперь делает **snapshot ключей под lock и отпускает lock до `.await`** — это *частично закрывает* старый класс проблем “lock across await”.
- При этом ряд “точечных” проблем остаётся неизменным: лишние копии входящих сообщений, `bin.to_vec()` при UTF-8, `sender.send(...)` под lock, drain+send под lock.

---

## 1) Что в audit остаётся верным относительно текущего `src`

### 1.1 Mutex contention в горячем пути входящих сообщений — **всё ещё есть**
В `handle_incoming()` маршрут по subscription-уведомлениям до сих пор держит `Mutex` во время `send()`:

- `let mut subs = route.lock().await;` … `sender.send(text)` — см. `src/ws_client.rs:681–687`.

Это полностью соответствует тезисам audit про contention на `subscriptions` (теперь это два мапа: `public_subscriptions` и `private_subscriptions`, но проблема идентична).

### 1.2 Полный JSON parse (`serde_json::Value`) на каждое сообщение — **всё ещё есть**
`handle_incoming()` продолжает делать `serde_json::from_str::<Value>(&text)` и дальше вытаскивает `id/channel_name` — т.е. “дерево Value” строится даже если достаточно envelope-полей.

В коде это видно по `use serde_json::Value` и месту разбора в `handle_incoming()` (см. `src/ws_client.rs`, блок обработки сообщений около `:648` и выше).

Audit-рекомендация “вместо полного Value — лёгкий envelope/селективный парсинг” остаётся актуальной.

### 1.3 Нет batching переподписок — **всё ещё актуально**
`resubscribe_all()` повторно подписывается “по одному каналу за RPC”:

- public: `send_rpc("public/subscribe", { "channels": [channel.clone()] })`
- private: аналогично

Это соответствует audit-пункту “батчинг переподписок при реконнекте”.

---

## 2) Что в коде уже улучшено (и audit должен это учитывать)

### 2.1 “Lock across await” в переподписке **частично исправлен в src**
В текущем `resubscribe_all()` делается snapshot ключей:

- public snapshot: `src/ws_client.rs:383–386`
- private snapshot: `src/ws_client.rs:398–401`

То есть lock держится только чтобы собрать `Vec<String>`, а `.await` делается уже **без lock**.

➡️ Если в каких-то audit-файлах всё ещё фигурируют примеры/текст, подразумевающие “держим lock и внутри цикла `.await`” для resubscribe — это теперь **устаревшая диагностика** для данного кода.

---

## 3) Что из “точечных оптимизаций” по-прежнему не отражено в коде (audit корректен, но статус = не исправлено)

Ниже — те самые “дешёвые” улучшения, которые audit/доп-ревью обычно относят к low-risk perf wins.

### 3.1 Лишняя копия `String` на `Message::Text` — **есть**
- `handle_incoming(text.to_string(), ...)` — `src/ws_client.rs:592`  
`text` уже `String`, копирование лишнее.

### 3.2 Лишняя копия `Vec<u8>` на `Message::Binary` — **есть**
- `String::from_utf8(bin.to_vec())` — `src/ws_client.rs:599`  
`bin` уже `Vec<u8>`, можно без `to_vec()`.

### 3.3 `pending_requests.drain()` + `tx.send(...)` под lock — **есть**
При разрыве соединения:

- `let mut pending = pending_requests.lock().await;`
- `for (_, tx) in pending.drain() { tx.send(...) }`

Это видно в `src/ws_client.rs:500–504`.  
Да, это “не горячий путь” (disconnect path), но при большом числе pending может давать заметную паузу и блокировать другие операции.

### 3.4 Подписочные `sender.send(text)` под lock — **есть**
См. `src/ws_client.rs:681–687` — актуально для high-rate notifications.

---

## 4) Где audit может быть неполным относительно обновлённого кода (новые/расширенные зоны)

### 4.1 Появился `instruments_cache: Mutex<HashMap<...>>`
В `WsClient` теперь есть `instruments_cache` под `Mutex` (см. `src/ws_client.rs:66` и места обращения вроде `:150`, `:177`).  
Audit-файлы, которые фокусируются только на subscriptions/pending, могут не упоминать это — но при частом обращении к `get_instrument()` (если оно в hot path) это может стать дополнительным contention-пунктом.

### 4.2 Логика состояния соединения содержит двойные lock-чтения
В `run_till_event()` есть шаблон:

- сравнение с `*self.current_connection_state.lock().await`
- затем сразу второй lock для записи

Это мелочь, но если часто вызывается, можно оптимизировать (например, одним lock-скоупом или atomic/ watch-only). Audit может не учитывать.

---

## 5) Консистентность audit-папки с учётом ваших прошлых ревью (`thalex_rust_sdk_audit_*`)

- В `audit` присутствуют ваши прошлые ревью (`thalex_rust_sdk_audit_recheck_report_v2/v3*`). Они в целом остаются релевантными как “история правок отчётов”.
- Текущий исходный код **не обязан** соответствовать тексту audit, но при сравнении видно:
  - часть рекомендаций (snapshot в resubscribe) “случайно” уже реализована;
  - ключевые узкие места (Mutex+send под lock, полный JSON parse) по-прежнему на месте.

---

## 6) Что бы я рекомендовал обновить в audit-тексте, чтобы он точно совпадал с новым `src`

1) **Явно отметить, что lock-across-await в resubscribe уже снят** (теперь проблема — batching, а не lock-скоуп).
2) **Отразить наличие двух subscription-map’ов** (`public_subscriptions` / `private_subscriptions`), чтобы примеры кода не выглядели “не из этой версии”.
3) Добавить короткий абзац про **`instruments_cache`** (новый потенциальный contention spot), даже если без глубокой проработки.

---

### Приложение: быстрый чек-лист “audit vs src” по топ-рекомендациям

- Mutex→DashMap/RwLock для subscriptions: **не реализовано**
- JSON envelope parsing вместо `Value`: **не реализовано**
- batching resubscribe: **не реализовано**
- snapshot keys before await (resubscribe): **реализовано**
- убрать лишние копии text/bin: **не реализовано**
- send вне lock для subs/pending: **не реализовано**

