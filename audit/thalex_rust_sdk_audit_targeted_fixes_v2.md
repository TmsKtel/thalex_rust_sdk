# thalex_rust_sdk — Targeted audit text fixes (v2)
*(Scope: **only** `audit/` docs. Do **not** change `src/`.)*

This v2 is tailored to the current `audit/` state in **`thalex_rust_sdk6.tgz`** and addresses what still remained inconsistent after v1.

---

## Summary of what still needs fixing (as of sdk6)

1) **`FINAL_REPORT_en.md`** still contains **fragile line-number references** (e.g. “lines 671, 682…”) and repeated “lock … await” notes.  
2) **`step_by_step_implementation_guide(.md|_en.md)`** still uses **old API/field names** (`self.subscriptions`, single `subscriptions` map) and mentions `run_single_connection`.  
3) **`implementation_examples(.md|_en.md)`** still contains old names (`self.subscriptions`, `run_single_connection`) and lacks a clear “illustrative only” banner.  
4) **`FILES_INDEX(.md|_en.md)`** still doesn’t clearly mark “most up-to-date” docs vs “illustrative examples”, and numbering is a bit messy.

---

## 1) `audit/FINAL_REPORT_en.md` — remove line numbers and stabilize wording

### 1.1 Replace the “Code Locations (current line numbers)” block
**Where:** around lines **95–99**.

Current snippet (for reference):
```text
  93: - At high message frequency (e.g., ticker with 100ms delay), locks create queues
  94: 
  95: **Code Locations (current line numbers):**
  96: - `handle_incoming`: lines 671, 682 - locks for accessing `pending_requests` and `public_subscriptions`/`private_subscriptions`
  97: - `send_rpc`: lines 205, 222 - locks when adding/removing requests
  98: - `subscribe_channel`: lines 274, 280 - locks when managing subscriptions
  99: - `instruments_cache`: lines 150, 167-170, 177 - locks when working with instrument cache
 100: 
 101: **Impact:**
```

**Replace with (drop line numbers entirely):**
```md
**Code Locations (stable identifiers):**
- `handle_incoming()` — per-message hot path; touches `pending_requests` and the subscription maps (`public_subscriptions` / `private_subscriptions`)
- `send_rpc()` — adds/removes entries in `pending_requests`
- `subscribe_channel()` / `unsubscribe_channel()` — updates subscription maps
- instrument cache access (e.g., `instruments_cache`) — used when parsing instrument-related payloads
```

**Why:** line numbers drift whenever upstream edits happen; function/field names stay stable.

### 1.2 Normalize the “lock across await” wording for resubscribe
Search for the note that implies lock is held during `.await` (or has ellipsis). Keep it factual and crisp:

**Replace with (suggested):**
```md
**Note:** In the current code, `resubscribe_all()` snapshots channel names under a lock and releases the lock **before** awaiting network sends. This avoids holding a lock across `.await` during re-subscription.
```

(If you already have a similar note, ensure it does not cite stale code paths like `run_single_connection`.)

---

## 2) `audit/step_by_step_implementation_guide.md` (RU) — remove `self.subscriptions` + `run_single_connection` references

### 2.1 Add a “two subscription maps” disclaimer near the top
**Where:** right after the introduction / before the first step.

**Add:**
```md
> **Важно:** В актуальном `ws_client.rs` используются **две** таблицы подписок: `public_subscriptions` и `private_subscriptions`.
> Примеры ниже, где фигурирует единая `subscriptions` / `self.subscriptions`, являются **иллюстративными** и должны быть адаптированы, выбрав соответствующую таблицу (public или private).
```

### 2.2 Replace `run_single_connection` references → `resubscribe_all()`
You currently have explicit references like:
- near the top: “найти функцию `run_single_connection` …”
- later in the file: “`run_single_connection()` …”

**Replace both** with:
```md
Открыть `src/ws_client.rs`, найти функцию `resubscribe_all()` (обработчик переподписок при реконнекте).
```

### 2.3 Replace `self.subscriptions.lock().await` examples (subscribe/unsubscribe steps)
**Where:** “Шаг 5: Изменить subscribe()” (around lines ~321–326) and “Шаг 6: Изменить unsubscribe()”.

Current snippet (subscribe example):
```text
 321: **Найти (строки 120-123):**
 322: ```rust
 323: {
 324:     let mut subs = self.subscriptions.lock().await;
 325:     subs.insert(channel.clone(), tx);
 326: }
 327: ```
 328: 
 329: **Заменить на:**
```

**Replace the code blocks with a map-specific template (public OR private):**
```md
```rust
// Выберите нужную таблицу подписок: public_subscriptions или private_subscriptions
{
    let mut subs = self.public_subscriptions.lock().await; // или self.private_subscriptions
    subs.insert(channel.clone(), tx);
}
```
```

And for unsubscribe:
```md
```rust
{
    let mut subs = self.public_subscriptions.lock().await; // или self.private_subscriptions
    subs.remove(&channel);
}
```
```

**Why:** `self.subscriptions` does not exist in the current codebase; using it in a “step-by-step” guide causes copy/paste failures.

---

## 3) `audit/step_by_step_implementation_guide_en.md` — same fixes (EN)

### 3.1 Add the disclaimer
Add near the top:
```md
> **Note:** The current `ws_client.rs` maintains **two** subscription maps: `public_subscriptions` and `private_subscriptions`.
> Any example below that refers to a single `subscriptions` / `self.subscriptions` map is **illustrative** and must be adapted by selecting the appropriate map (public or private).
```

### 3.2 Replace `run_single_connection` references → `resubscribe_all()`
Search and replace the “open … find `run_single_connection` …” instructions with `resubscribe_all()`.

### 3.3 Replace `self.subscriptions` code blocks
Search for blocks like:
```rust
let mut subs = self.subscriptions.lock().await;
```
and replace with:
```rust
let mut subs = self.public_subscriptions.lock().await; // or self.private_subscriptions
```

Also replace `self.subscriptions.remove(...)` with the map-specific version.

---

## 4) `audit/implementation_examples.md` (RU) — add banner + de-risk misleading identifiers

### 4.1 Add an “Illustrative examples” banner at the very top
Add as the first lines:
```md
> **Важно:** Примеры в этом файле иллюстративны. В актуальном коде используются `public_subscriptions` и `private_subscriptions`.
> Если пример использует единую `subscriptions` / `self.subscriptions`, адаптируйте его, выбрав соответствующую таблицу (public или private).
```

### 4.2 Replace `run_single_connection` mentions → `resubscribe_all()`
Search `run_single_connection` and replace with `resubscribe_all()` where it describes reconnect re-subscription.

### 4.3 Replace `self.subscriptions` references in examples
Replace `self.subscriptions` with `self.public_subscriptions` and add an inline “or private” comment where needed.

---

## 5) `audit/implementation_examples_en.md` — same (EN)

### 5.1 Add the banner
Add at the top:
```md
> **Important:** Examples in this file are illustrative. The current code uses `public_subscriptions` and `private_subscriptions`.
> If an example refers to a single `subscriptions` / `self.subscriptions` map, adapt it by selecting the appropriate map (public or private).
```

### 5.2 Replace `run_single_connection` → `resubscribe_all()`
Search and replace where it describes reconnect behavior.

### 5.3 Replace `self.subscriptions` references
Replace with `self.public_subscriptions` and add “or private” where appropriate.

---

## 6) `audit/FILES_INDEX.md` and `audit/FILES_INDEX_en.md` — navigation + numbering

### 6.1 Add “Most up-to-date documents” section
Add near the top (after the intro):

```md
## Most up-to-date (read first)
- `FINAL_REPORT.md` / `FINAL_REPORT_en.md` — primary report
- `performance_analysis.md` / `performance_analysis_en.md` — supporting analysis
```

### 6.2 Add “Illustrative examples” section
Add:

```md
## Illustrative / examples (may require adaptation)
- `implementation_examples.md` / `implementation_examples_en.md`
- `step_by_step_implementation_guide.md` / `step_by_step_implementation_guide_en.md`
```

### 6.3 Fix numbering
If the file currently has duplicated numbering (e.g., two “7.”), renumber sequentially.

---

## 7) Optional hygiene: remove editor artifacts
If not required for delivery, remove:
- `a.txt`, `a2.txt`, `a3.txt`, `a4.txt`, `update.txt`

---

## 8) Post-edit validation checklist

- [ ] `FINAL_REPORT_en.md` contains **no** “lines …” references; uses stable identifiers instead.
- [ ] `step_by_step_*` contains **no** `self.subscriptions` and does **not** instruct to edit `run_single_connection`.
- [ ] `implementation_examples*` starts with a banner that examples are illustrative and references the two maps.
- [ ] `FILES_INDEX*` clearly marks “read first” vs “illustrative” docs.
- [ ] No misleading JSON-RPC pre-checks that assume string `id` (if mentioned, it should be `"id":123`, not `"id":"123"`).

---

*End of v2.*
