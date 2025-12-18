# Thalex Rust SDK Code Audit

## Project Overview

Thalex Rust SDK is a client library for working with the Thalex exchange WebSocket API. The project provides an asynchronous client for connecting to the WebSocket API, executing RPC requests, and subscribing to real-time data channels.

## Architecture and Components

### 1. Main Modules

#### `src/lib.rs`
Main library module, exports:
- `models` - data models
- `ws_client` - WebSocket client

#### `src/ws_client.rs`
Main module containing the WebSocket client implementation.

**Key Components:**

- **`WsClient`** - public client API
  - `connect_default()` / `connect()` - WebSocket connection
  - `call_rpc()` - JSON-RPC request execution
  - `subscribe()` / `unsubscribe()` - channel subscription/unsubscription
  - `shutdown()` - graceful shutdown

- **`connection_supervisor`** - connection supervisor
  - Automatic reconnection on connection loss
  - Re-subscription to active channels after reconnection
  - Connection error handling with exponential delay (3 seconds)

- **`run_single_connection`** - single connection handling
  - Incoming message processing (Text, Binary, Ping/Pong, Close)
  - Command sending through channel
  - Shutdown signal handling

- **`handle_incoming`** - incoming message handling
  - JSON parsing
  - RPC response routing by ID
  - Subscription routing by channel_name

#### `src/models/`
Data models generated from OpenAPI specification:

- **`Delay`** - enum for update intervals (100ms, 200ms, 500ms, 1000ms, 5000ms, 60000ms, raw)
- **`TickerData`** - ticker data structure with fields:
  - Bid/ask prices and volumes
  - Last trade price
  - Mark price and timestamp
  - IV, delta, index, forward (for options)
  - 24-hour volumes and statistics
  - Funding rate (for perpetuals)
  - Open interest, price collars
- **`TickerResponse`** - subscription response wrapper with channel_name and notification

### 2. Data Flows

#### RPC Requests (request-response)
1. Client calls `call_rpc(method, params)`
2. Unique ID is generated via `AtomicU64`
3. `oneshot::channel` is created for response
4. Request is added to `pending_requests` HashMap
5. Message is sent via WebSocket
6. When response with matching ID is received, response is sent through oneshot channel
7. 30-second timeout for response waiting

#### Subscriptions (pub-sub)
1. Client calls `subscribe(channel, callback)`
2. `mpsc::unbounded_channel` is created for channel
3. Subscription is added to `subscriptions` HashMap
4. Subscription command is sent to server
5. Separate task is created for callback handling
6. When message with `channel_name` is received, it's sent to corresponding channel
7. Callback is invoked in separate task

### 3. Connection Management

**Connection Supervisor:**
- Infinite reconnection loop
- On connection loss:
  - All pending RPC requests are marked as failed
  - Active subscriptions are preserved
  - After reconnection, all subscriptions are automatically restored
- Reconnection delay: 3 seconds (fixed)

**Single Connection Handling:**
- Uses `tokio::select!` for multiplexing:
  - Shutdown signals
  - Send commands
  - Incoming WebSocket messages
- Handles Ping/Pong for keepalive
- Gracefully closes connection on shutdown

## Technologies Used

- **tokio** - asynchronous runtime
- **tokio-tungstenite** - WebSocket client
- **futures-util** - utilities for working with futures
- **serde/serde_json** - JSON serialization/deserialization
- **log/simple_logger** - logging

## Design Patterns

1. **Supervisor Pattern** - supervisor manages connection lifecycle
2. **Actor Pattern** - commands are sent through channels
3. **Pub-Sub Pattern** - channel data subscriptions
4. **Request-Response Pattern** - RPC calls

## Usage Examples

See `examples/subscribe.rs`:
- WebSocket connection
- RPC request execution `public/instruments`
- Subscription to channel `ticker.BTC-PERPETUAL.100ms`
- Wait 60 seconds
- Graceful shutdown

