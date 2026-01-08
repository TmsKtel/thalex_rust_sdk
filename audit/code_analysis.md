# Аудит кода Thalex Rust SDK

## Обзор проекта

Thalex Rust SDK - это клиентская библиотека для работы с WebSocket API биржи Thalex. Проект предоставляет асинхронный клиент для подключения к WebSocket API, выполнения RPC-запросов и подписки на каналы данных в реальном времени.

## Архитектура и компоненты

### 1. Основные модули

#### `src/lib.rs`
Главный модуль библиотеки, экспортирует:
- `models` - модели данных
- `ws_client` - WebSocket клиент
- `channels` - модуль для подписок (namespaces: market_data, accounting, conditional, etc.)
- `rpc` - модуль для RPC запросов (namespaces: market_data, trading, accounting, etc.)
- `types` - типы данных
- `utils` - утилиты

#### `src/ws_client.rs`
Основной модуль, содержащий реализацию WebSocket клиента.

**Ключевые компоненты:**

- **`WsClient`** - публичный API клиента
  - `new()` / `new_public()` / `from_env()` - создание клиента
  - `rpc()` - доступ к RPC методам (через модуль `rpc`)
  - `subscriptions()` - доступ к подпискам (через модуль `channels`)
  - `send_rpc()` - выполнение JSON-RPC запросов (внутренний метод)
  - `subscribe_channel()` - подписка на канал (внутренний метод)
  - `login()` - аутентификация
  - `shutdown()` - корректное завершение работы

- **`connection_supervisor`** - супервизор соединения
  - Автоматическое переподключение при разрыве соединения
  - Повторная подписка на активные каналы после переподключения
  - Обработка ошибок подключения с экспоненциальной задержкой (3 секунды)

- **`resubscribe_all`** - переподписка при реконнекте (snapshot каналов под lock, send без lock)
  - Обработка входящих сообщений (Text, Binary, Ping/Pong, Close)
  - Отправка команд через канал
  - Обработка shutdown сигналов

- **`handle_incoming`** - обработка входящих сообщений
  - Парсинг JSON
  - Маршрутизация RPC-ответов по ID
  - Маршрутизация подписок по channel_name

#### `src/models/`
Модели данных, сгенерированные из OpenAPI спецификации:

- **`Delay`** - enum для интервалов обновления (100ms, 200ms, 500ms, 1000ms, 5000ms, 60000ms, raw)
- **`TickerData`** - структура данных тикера с полями:
  - Цены bid/ask и их объемы
  - Последняя цена сделки
  - Mark price и timestamp
  - IV, delta, index, forward (для опционов)
  - Объемы и статистика за 24 часа
  - Funding rate (для перпетуалов)
  - Open interest, price collars
- **`TickerResponse`** - обертка для ответа подписки с channel_name и notification

### 2. Потоки данных

#### RPC запросы (request-response)
1. Клиент вызывает методы через `client.rpc().*()` (например, `client.rpc().market_data().instruments()`)
2. Внутри вызывается `send_rpc(method, params)`
3. Генерируется уникальный ID через `AtomicU64`
4. Создается `oneshot::channel` для ответа
5. Запрос добавляется в `pending_requests` HashMap
6. Сообщение отправляется через WebSocket
7. При получении ответа с соответствующим ID, ответ отправляется через oneshot channel
8. Таймаут определяется ожиданием ответа через oneshot channel

#### Подписки (pub-sub)
1. Клиент вызывает методы через `client.subscriptions().*()` (например, `client.subscriptions().market_data().ticker(...)`)
2. Внутри вызывается `subscribe_channel(scope, channel, callback)`
3. Создается `mpsc::unbounded_channel` для канала
4. Подписка добавляется в `public_subscriptions` или `private_subscriptions` HashMap
5. Отправляется команда подписки на сервер через RPC
6. Создается отдельная задача для обработки callback
7. При получении сообщения с `channel_name`, оно отправляется в соответствующий канал
8. Callback вызывается в отдельной задаче

### 3. Управление соединением

**Супервизор соединения:**
- Бесконечный цикл переподключения
- При разрыве соединения:
  - Все pending RPC запросы помечаются как failed
  - Активные подписки сохраняются
  - После переподключения автоматически восстанавливаются все подписки
- Задержка переподключения: 3 секунды (фиксированная)

**Обработка одного соединения:**
- Использует `tokio::select!` для мультиплексирования:
  - Shutdown сигналы
  - Команды на отправку
  - Входящие WebSocket сообщения
- Обрабатывает Ping/Pong для keepalive
- Корректно закрывает соединение при shutdown

## Используемые технологии

- **tokio** - асинхронный runtime
- **tokio-tungstenite** - WebSocket клиент
- **futures-util** - утилиты для работы с futures
- **serde/serde_json** - сериализация/десериализация JSON
- **log/simple_logger** - логирование

## Паттерны проектирования

1. **Supervisor Pattern** - супервизор управляет жизненным циклом соединения
2. **Actor Pattern** - команды отправляются через каналы
3. **Pub-Sub Pattern** - подписки на каналы данных
4. **Request-Response Pattern** - RPC вызовы

## Примеры использования

См. примеры в папке `examples/`:
- `subscribe_ticker.rs` - подписка на ticker
- `subscribe_account.rs` - подписка на account данные
- `simple_quoter.rs` - простой пример использования
- `collect_all_trades.rs` - сбор всех сделок
- `ohlc_streamer.rs` - поток OHLC данных

Типичный workflow:
1. Создание клиента: `WsClient::new()` или `WsClient::from_env()`
2. Логин: `client.login().await?`
3. RPC запросы: `client.rpc().market_data().instruments().await?`
4. Подписки: `client.subscriptions().market_data().ticker(instrument, delay, callback).await?`
5. Ожидание событий: `client.run_till_event().await`
6. Корректное завершение: `client.shutdown()`

