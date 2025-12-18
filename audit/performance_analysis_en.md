# Performance Analysis and Bottlenecks

## Identified Bottlenecks

### 1. Mutex Locks in Hot Paths

**Problem:**
- In `handle_incoming` function (lines 334-373), Mutex lock occurs for every incoming message
- This is a critical path, as all messages pass through this function
- At high message frequency (e.g., ticker with 100ms delay), locks can create queues

**Locations:**
- `handle_incoming`: lines 349, 360 - locks for accessing `pending_requests` and `subscriptions`
- `call_rpc`: lines 81, 98 - locks when adding/removing requests
- `subscribe/unsubscribe`: lines 121, 149 - locks when managing subscriptions

**Impact:**
- Delay in processing each message due to lock waiting
- Potential degradation with large number of concurrent RPC requests
- Contention between incoming message processing and subscription management

### 2. Excessive String Cloning

**Problem:**
- In `subscribe` (line 115): `channel.to_string()` creates a new string
- In `handle_incoming` (line 335): `text: String` is accepted by value but then passed further
- In `run_single_connection` (line 264): `msg.to_string()` creates a string for each channel on reconnection
- In `connection_supervisor` (line 218): new string is created for each failed request

**Impact:**
- Unnecessary memory allocations
- Data copying instead of moving
- Additional load on allocator (allocations are still expensive in Rust)

### 3. JSON Parsing for Every Message

**Problem:**
- In `handle_incoming` (line 339), every incoming text is parsed into `serde_json::Value`
- Even if message doesn't require full parsing (e.g., only need to check for "id" or "channel_name" field)

**Impact:**
- JSON parsing is CPU-intensive operation
- At high message frequency, this can become a bottleneck
- Excessive parsing if only certain fields are needed

### 4. HashMap Operations in Critical Path

**Problem:**
- HashMap lookup under lock for every message
- With large number of subscriptions or pending requests, lookup can slow down
- Using `String` as key requires string hashing

**Impact:**
- O(1) on average, but can degrade on collisions
- String hashing is relatively expensive operation

### 5. No Batching on Re-subscription

**Problem:**
- In `run_single_connection` (lines 257-266), on reconnection, separate message is sent for each channel
- Could send one command with array of all channels

**Impact:**
- More network round-trips
- More JSON serialization
- Slower subscription recovery

### 6. Fixed Reconnection Delay

**Problem:**
- In `connection_supervisor` (lines 232, 239), fixed 3-second delay is used
- No exponential backoff
- No jitter to prevent thundering herd

**Impact:**
- Can create excessive load on network problems
- Slower recovery on temporary issues

### 7. Separate Task Creation for Each Callback

**Problem:**
- In `subscribe` (line 135), separate tokio task is created for each subscription
- With large number of subscriptions, this creates many tasks

**Impact:**
- Additional overhead for task management
- More context switches

### 8. No Buffer Pool for Reuse

**Problem:**
- Every message creates new strings and buffers
- No memory reuse

**Impact:**
- More allocations
- More pressure on allocator

## Metrics for Measurement

Recommended to add metrics for:
1. Mutex lock hold time
2. Messages per second
3. Message processing latency
4. Allocation count
5. CPU usage under high load

## Optimization Priorities

### High Priority:
1. Lock optimization in `handle_incoming` (using RwLock or lock-free structures)
2. Subscription batching on reconnection
3. JSON parsing optimization (lazy parsing or streaming parser)

### Medium Priority:
4. Reducing string cloning
5. Exponential backoff for reconnection
6. Using more efficient data structures (e.g., `DashMap` for concurrent access)

### Low Priority:
7. Buffer pool
8. Callback task management optimization

