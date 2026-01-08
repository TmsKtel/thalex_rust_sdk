# thalex_rust_sdk — Targeted audit text fixes (v1)
*(Only `audit/` files. Do **not** change `src/`.)*

This document lists **surgical text edits** to make the audit reports consistent with the **current** `src/ws_client.rs` structure (notably `public_subscriptions` / `private_subscriptions`, and `resubscribe_all()`), and to remove stale references like `run_single_connection` line ranges that no longer match.

---

## 0) Context and goal

**Goal:** Ensure the audit text does not mislead readers by:
- referencing outdated code locations / line numbers,
- using old field names (`subscriptions`) that no longer exist,
- claiming “lock across await” in resubscribe is still present, when the code already snapshots keys and releases locks.

**Non-goals:** No architecture changes, no SDK refactor suggestions beyond what’s already in audit—this is purely **documentation alignment**.

---

## 1) `audit/FINAL_REPORT.md` — required edits

### 1.1 Replace stale “resubscribe in run_single_connection (lines …)” section

**Find (typical patterns to search):**
- `run_single_connection`
- `lines 257–266`
- `let subs = subscriptions.lock().await;`
- any paragraph claiming `ws.send(...).await` happens **while holding** the subscriptions lock

**Replace with (suggested text):**
> **Re-subscription on reconnect**  
> In the current implementation, re-subscription is handled in `resubscribe_all()`. The code snapshots channel names under a lock and releases the lock **before** awaiting network sends. This avoids holding a lock across `.await` (previously a risk in older variants).  
> Remaining optimization opportunity: **batch** re-subscription requests (e.g., subscribe to multiple channels in a single message when supported by the API) to reduce the number of round-trips.

**Why:** `subscriptions` and the `run_single_connection`-based resubscribe path are no longer accurate.

### 1.2 Update terminology: `subscriptions` → `public_subscriptions` / `private_subscriptions`

**Find:** occurrences of `subscriptions` where it refers to internal maps.

**Replace with:** `public_subscriptions` and/or `private_subscriptions` depending on context. If the text is generic, use:
> “subscription maps (`public_subscriptions` and `private_subscriptions`)”

**Why:** prevents copy/paste errors and keeps audit consistent with code.

### 1.3 Line-number references: remove or soften

**Find:** exact line ranges (“lines X–Y”) for sections that are known to drift frequently.

**Replace with:** function/identifier references, e.g.:
- `handle_incoming()`
- `resubscribe_all()`
- `pending_requests`
- `public_subscriptions` / `private_subscriptions`

**Why:** line numbers are fragile and tend to become wrong after upstream changes.

---

## 2) `audit/FINAL_REPORT_en.md` — mirror the same edits

Apply the same changes as in §1, with the English equivalents.

Suggested replacement paragraph (EN):

> **Re-subscription on reconnect**  
> In the current implementation, re-subscription is performed in `resubscribe_all()`. Channel names are snapshotted under a lock and the lock is released **before** awaiting network sends, so the code avoids holding a lock across `.await` (a risk in older variants).  
> Remaining opportunity: **batch** re-subscription requests (when the API supports it) to reduce round-trips.

---

## 3) `audit/step_by_step_implementation_guide.md` — fix non-existent `self.subscriptions` examples

### 3.1 Replace `self.subscriptions` examples

**Find:** code examples using:
- `self.subscriptions`
- `subscriptions.lock().await` as a single map

**Replace with one of these approaches (choose A or B):**

#### Option A (preferred): update examples to show both maps
Replace examples to demonstrate both `public_subscriptions` and `private_subscriptions`, e.g.:

```rust
let pub_sender = {
    let subs = self.public_subscriptions.lock().await;
    subs.get(channel).cloned()
};

if let Some(tx) = pub_sender {
    let _ = tx.send(msg);
}
```

And if needed, same for private.

#### Option B (minimal text-only): add a prominent disclaimer at the top of the guide
Add near the top:

> **Note:** The current codebase maintains two subscription maps: `public_subscriptions` and `private_subscriptions`.  
> Any example in this guide that refers to a single `subscriptions` map is illustrative and must be adapted by choosing the appropriate map.

**Why:** the guide currently implies a single `subscriptions` field, which may not exist anymore.

---

## 4) `audit/step_by_step_implementation_guide_en.md` — mirror the same edits

Apply §3 analogously in English.

Suggested disclaimer (EN):

> **Note:** The current codebase maintains two subscription maps: `public_subscriptions` and `private_subscriptions`.  
> Any example in this guide referring to a single `subscriptions` map is illustrative and must be adapted by selecting the appropriate map.

---

## 5) `audit/implementation_examples.md` and `_en.md` — prevent copy/paste failures

### 5.1 Add a “Examples are illustrative” banner at the top

Add at the very top:

> **Important:** Examples in this file are illustrative. The current codebase uses `public_subscriptions` and `private_subscriptions`.  
> If an example refers to a single `subscriptions` map, adapt it by selecting the appropriate map.

(English equivalent in `_en.md`.)

### 5.2 Update only the most misleading examples (minimum set)
Search for examples that include:
- `self.subscriptions`
- `let subs = subscriptions.lock().await;` followed by `await` network send
- string-based JSON-RPC id pre-check like `"id":"`

Update them to:
- refer to `public_subscriptions/private_subscriptions`,
- avoid showing “lock across await”,
- use numeric id fast-check `"id":` (no quotes) if a fast-check is shown.

---

## 6) `audit/FILES_INDEX.md` — small hygiene + accuracy improvements

### 6.1 Fix numbering / duplicates
**Find:** duplicated section numbers (e.g., two “7.”).  
**Fix:** renumber sequentially.

### 6.2 Mark “Most up-to-date” files explicitly
Add a short section:

> **Most up-to-date documents:**  
> - `FINAL_REPORT.md` / `FINAL_REPORT_en.md` (primary)  
> - `performance_analysis.md` / `performance_analysis_en.md` (supporting)

And optionally mark older or illustrative docs:

> **Illustrative / examples (may require adaptation):**  
> - `implementation_examples*.md`

---

## 7) Optional cleanup: remove stray editor artifacts
If files like `a.txt`, `a2.txt`, `update.txt` are not intended for the customer deliverable, remove them to avoid confusion.

---

## 8) Quick validation checklist (after edits)

- [ ] No mention of `self.subscriptions` unless clearly labeled as illustrative.
- [ ] Resubscribe text references `resubscribe_all()` and does **not** claim “lock across await” still exists.
- [ ] If JSON-RPC `id` fast-check is shown, it uses `"id":` (numeric id) not `"id":"`.
- [ ] `FINAL_REPORT` does not cite stale line numbers that no longer match the current code.
- [ ] `FILES_INDEX.md` clearly points to the primary report(s).

---

*End of document.*
