
# thalex_rust_sdk — Targeted audit text fixes (v3)
*(Scope: **only** `audit/` docs. Do **not** change `src/`.)*

This v3 is tailored to `thalex_rust_sdk7.tgz` and is intentionally **mechanical**: copy/paste the replacement blocks exactly to close the remaining issues from all previous reviews (v1 + v2).

---

## What’s still failing the “all previous audit remarks are accounted for” gate (sdk7)

- Some **EN** docs still contain fragile `line … / lines …` references.
- Several docs still reference the **obsolete** `run_single_connection` path (audit must describe current reconnect/resubscribe implementation as `resubscribe_all()` and the “reader loop / incoming message handling” instead).
- `implementation_examples(.md|_en.md)` still contains **copy‑pasteable code** using non-existent `self.subscriptions` (banner alone is not enough).

---

# 1) `audit/FINAL_REPORT_en.md` — remove remaining line reference

### 1.1 Remove the “(line 501-503)” fragment in the bottlenecks list
**Find this exact bullet (currently around line 149):**
```text
 146: - Separate task creation for ...k on connection loss (line 501-503)
 150: 
 151: ---
 152: 
```

**Replace the specific line with:**
```md
- `pending_requests.drain()` + `send()` under lock on connection loss (see connection error handling / cleanup path)
```

**Why:** Avoid fragile line numbers in the primary EN report.

---

# 2) `audit/optimization_recommendations_en.md` — remove `run_single_connection` + all line numbers

## 2.1 Top “Problem” paragraph: replace `run_single_connection (lines …)` phrasing
**Find the first paragraph that begins like:**
```text
   4: 
   5: ### Problem
   6: In `run_si... → `String::from_utf8(bin.to_vec())` - unnecessary buffer copy
```

**Replace the intro sentence with:**
```md
In the WebSocket reader loop (handling `Message::Text` / `Message::Binary` before calling `handle_incoming()`), unnecessary allocations occur:
- `handle_incoming(text.to_string(), ...)` — unnecessary `String` clone
- `String::from_utf8(bin.to_vec())` — unnecessary buffer copy
```

*(Keep the rest of the section intact.)*

## 2.2 Remove line-number headings for locks
You currently have headings like:
- `**For subscriptions (line 682-689):**`
- `**For pending_requests (line 671-673):**`
- `**For pending_requests.drain() on connection loss (line 501-503):**`

**Replace them with line-free headings:**
```md
**For subscription maps (`public_subscriptions` / `private_subscriptions`):**
```
```md
**For `pending_requests`:**
```
```md
**For `pending_requests.drain()` on connection loss:**
```

(Do **not** change the code blocks underneath unless they mention `subscriptions` instead of public/private.)

Example location (one of the headings) currently looks like:
```text
  34: ### Solution A: Send Outside Lock (Q...89):**
  37: ```rust
  38: // Current: lock held during send
```

## 2.3 Resubscribe note: remove “(lines 383-386, …)” from the fixed-issue note
You currently have:
```text
 269: ## 4. Subscription Batching
 270: ...d of one request with all channels.
 272: 
 273: ### Problem
```

**Replace that note line with:**
```md
**Note:** ✅ In current code, the “lock across await” issue in `resubscribe_all()` is already fixed — channel names are snapshotted under lock and the lock is released before awaiting sends. The remaining issue is batching (sending one channel at a time instead of one request with all channels).
```

## 2.4 Replace the stale code comment `// In run_single_connection, lines 257-266`
You currently have a snippet like:
```text
 278: 
 279: ```rust
 280: // In run_si... make snapshot under lock
 282: let channels: Vec<String> = {
```

**Replace the comment line with:**
```rust
// In `resubscribe_all()`: take a snapshot of channels under lock, then send without holding the lock
```

**Why:** `run_single_connection` is no longer the right anchor, and line ranges drift.

---

# 3) `audit/optimization_recommendations.md` (RU) — remove `run_single_connection` anchors

## 3.1 Replace “В run_single_connection …” phrasing at the top
Current snippet around line 6:
```text
   4: 
   5: ### Проблема
   6: В `run_sing... `String::from_utf8(bin.to_vec())` - лишнее копирование буфера
```

**Replace the first sentence with:**
```md
В цикле чтения WebSocket (обработка `Message::Text` / `Message::Binary` перед вызовом `handle_incoming()`), есть лишние аллокации:
- `handle_incoming(text.to_string(), ...)` — лишний клон `String`
- `String::from_utf8(bin.to_vec())` — лишнее копирование буфера
```

## 3.2 Replace the stale comment `// В run_single_connection, строки 257-266`
Current snippet around line 260:
```text
 258: 
 259: ```rust
 260: // В run_sin... делаем snapshot под lock
 262: let channels: Vec<String> = {
```

**Replace that comment line with:**
```rust
// В `resubscribe_all()`: делаем snapshot каналов под lock, затем отправляем без удержания lock
```

---

# 4) `audit/performance_analysis_en.md` — remove `run_single_connection` + line references

You currently have bullets like (around lines 33–36):
```text
  31: 
  32: **Problem:**
  33: - In `run...hannel` (line 309): `channel.to_string()` creates a new string
```

## 4.1 Replace that bullet list with (line-free, function/behavior based)
```md
**Problem:**
- In the WebSocket reader loop, `handle_incoming(text.to_string(), ...)` performs an unnecessary `String` clone.
- In the binary branch, `String::from_utf8(bin.to_vec())` performs an unnecessary buffer copy.
- In subscribe/unsubscribe handling, `channel.to_string()` may allocate when the input is already an owned `String` (verify actual call sites).
```

**Important:** remove explicit `line 592/599/309` references here.

---

# 5) `audit/code_analysis.md` + `audit/code_analysis_en.md` — stop listing `run_single_connection` as a key function

## 5.1 English file
Current snippet (around line 39):
```text
  37:   - Connection error handling..., Ping/Pong, Close)
  41:   - Command sending through channel
```

**Change the entry:**
- Replace: ``**`run_single_connection`** - single connection handling``
- With: ``**`resubscribe_all`** - re-subscription flow on reconnect (snapshots channels under lock, sends without holding lock)``

Also, if you have a “key functions” list, add `handle_incoming()` explicitly as the per-message hot path.

## 5.2 Russian file
Similarly replace:
- `**run_single_connection** - обработка одного соединения`
with
- `**resubscribe_all** - переподписка при реконнекте (snapshot каналов под lock, send без lock)`

---

# 6) `audit/thalex_rust_sdk_performance_reaudit_2025_en.md` — remove stale `run_single_connection` anchors

## 6.1 Replace the comment `// In run_single_connection, line 590-596`
Current snippet around line 205:
```text
 203: 
 204: ```rust
 205: // In run_...Some(Ok(Message::Text(text))) => {
 207:     handle_incoming(
```

**Replace the comment line with:**
```rust
// In the WebSocket reader loop (handling `Message::Text`) before calling `handle_incoming()`
```

## 6.2 Phase checklist: remove “in run_single_connection” phrasing
Current snippet around line 353:
```text
 351: 
 352: ### Phase 1: Quick Wins (...in.to_vec())`
 355: 3. ✅ Add fast key checking before parsing
```

Replace:
- `✅ Remove text.to_string() in run_single_connection`
with:
- `✅ Remove unnecessary text cloning in the WebSocket reader loop (pass `text` directly into `handle_incoming()`)`

*(Do the same for RU re-audit if it contains the same anchor text.)*

---

# 7) `audit/implementation_examples.md` and `_en.md` — banner exists ✅, but code must not use `self.subscriptions`

Example location (EN, around line 251):
```text
 248: **Before:**
 249: ```rust
 250...252:     subs.insert(channel.clone(), tx);
 253: }
 254: ```
```

Example location (RU, around line 251):
```text
 248: **До:**
 249: ```rust
 250: {\...252:     subs.insert(channel.clone(), tx);
 253: }
 254: ```
```

## 7.1 Minimal safe fix: replace *every* `self.subscriptions` block with an explicit “choose map” template

**Before (pattern):**
```rust
{
    let mut subs = self.subscriptions.lock().await;
    subs.insert(channel.clone(), tx);
}
```

**After:**
```rust
{
    // Choose the appropriate map: public_subscriptions OR private_subscriptions
    let mut subs = self.public_subscriptions.lock().await; // or self.private_subscriptions
    subs.insert(channel.clone(), tx);
}
```

And for remove:

**Before (pattern):**
```rust
{
    let mut subs = self.subscriptions.lock().await;
    subs.remove(&channel);
}
```

**After:**
```rust
{
    // Choose the appropriate map: public_subscriptions OR private_subscriptions
    let mut subs = self.public_subscriptions.lock().await; // or self.private_subscriptions
    subs.remove(&channel);
}
```

**Occurrence checklist (sdk7):**
- `implementation_examples.md`: occurrences at ~251, 266, 580, 606, 632
- `implementation_examples_en.md`: occurrences at ~251/258/266/273 and ~580/606/632

---

# 8) Post-edit validation checklist (sdk7)

- [ ] `FINAL_REPORT_en.md` contains **no** `(line …)` fragments anywhere.
- [ ] `optimization_recommendations_en.md` contains **no** `run_single_connection` and **no** `line/lines …` references.
- [ ] `optimization_recommendations.md` contains **no** `run_single_connection` references or comments.
- [ ] `performance_analysis_en.md` contains **no** `run_single_connection` and no numeric line refs.
- [ ] `code_analysis(.md|_en.md)` lists `handle_incoming()` and `resubscribe_all()`, not `run_single_connection`.
- [ ] `implementation_examples(.md|_en.md)` has **zero** `self.subscriptions` inside code blocks.
- [ ] JSON-RPC `id` is only referenced as numeric (`"id":123`), and `"id":"123"` appears only as a *counterexample*.

---

*End of v3.*
