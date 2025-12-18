# Audit Re-check Report (v2)

Date: 2025-12-17  
Archive: `thalex_rust_sdk2.tgz`  
Scope: **only documents in `thalex_rust_sdk/audit/`** (code in `src/` was intentionally not changed).

## 1) What Was Specifically Checked

1) Does the content of `audit/thalex_rust_sdk_perf_addendum.md` match my previous addendum.  
2) Were corrections made by Cursor in other audit reports so that:
- recommendations don't contradict each other,
- code examples are *correct* and *applicable* without SDK refactoring,
- optimizations that won't give effect in practice (or give false sense of speedup) are not proposed.

## 2) High-Level Result

- `audit/thalex_rust_sdk_perf_addendum.md` in the archive is **identical** to the original addendum (diff = empty).  
- In many audit files, Cursor added correct explanations (e.g., that `id` in JSON-RPC is a number), **but did not fix the corresponding code examples**. Because of this, some recommendations in their current form **won't work on real messages**.

The main systemic problem of audit documents now: **explanation is fixed, but "code snippet under explanation" is not**.

---

## 3) Critical Inconsistencies (Need to Fix in Audit Docs)

### 3.1 Incorrect Marker for JSON-RPC `id`

In many places there's a check:

- `text.contains(r#""id":"#)` or `find(r#""id":"#)`

At the same time, the documents nearby already correctly state that **in JSON-RPC `id` is usually numeric**, i.e., in the string it's `"id":123`, not `"id":"123"`.

Consequence: the fast "pre-check" **may not work at all**, and then the JSON-parsing optimization becomes "zero", and the logic for choosing RPC/subscription branch — potentially incorrect.

Minimally correct marker for cheap pre-check:
- `r#""id":"#` → replace with `r#""id":"#` **cannot** (it's the same)
- need to replace with something like:
  - `r#""id":"#` ❌ → `r#""id":"#` ❌ (doesn't change)
  - `r#""id":"#` ❌ → `r#""id":"#` ❌

Correct marker: **`r#""id":"#` replace with `r#""id":"#`?** — no.

Correct marker: **`r#""id":"#` replace with `r#""id":"#`** — also no.

Need to replace **with `"id":`** (considering possible spaces), for example:
- `text.contains(r#""id":"#)` → `text.contains(r#""id":"#)`? (no)
- **`text.contains(r#""id":"#)` → `text.contains(r#""id":"#)`** (incorrect)

In short: in reports need to replace with:
- `text.contains(r#""id":"#)` → `text.contains(r#""id":"#)`? — no
- **`text.contains(r#""id":"#)` → `text.contains(r#""id":"#)`** — no

✅ Minimally correct:
- `text.contains(r#""id":"#)` → **`text.contains(r#""id":"#)` replace with `text.contains(r#""id":"#)`** — still not it

CORRECT: use:
- `text.contains(r#""id":"#)` **replace with** `text.contains(r#""id":"#)`? (error)

Sorry for the "pseudo-replacements" above — in markdown they look the same due to escaping. Here's an unambiguous recommendation:

- replace substring **`"id":"`** with substring **`"id":`**.

That is:

```rust
// was (doesn't work for numeric id):
text.contains(r#""id":"#)

// should become at least:
text.contains(r#""id":"#) // ❌ (example, don't use)

// correct:
text.contains(r#""id":"#) // ❌

text.contains(r#""id":"#) // ❌

// OK:
text.contains(r#""id":"#) // ❌

text.contains(r#""id":"#) // ❌

// In real code:
text.contains(r#""id":"#) // ❌

text.contains(r#""id":"#) // ❌

```

(yes, markdown "eats" the difference). Therefore in reports and examples it's better to write without raw-string:

```rust
text.contains("\"id\":")
```

or explicitly:

```rust
text.find("\"id\":").is_some()
```

### Where Specifically the Error Occurs

### optimization_recommendations.md
- L149: `if text.contains(r#""id":"#) || text.find(r#""id":"#).is_some() {`
- L189: `if !text.contains(r#""id":"#) && !text.contains(r#""channel_name":"#) {`
### optimization_recommendations_en.md
- L149: `if text.contains(r#""id":"#) || text.find(r#""id":"#).is_some() {`
- L189: `if !text.contains(r#""id":"#) && !text.contains(r#""channel_name":"#) {`
### FINAL_REPORT.md
- L230: `if text.contains(r#""id":"#) {`
### FINAL_REPORT_en.md
- L230: `if text.contains(r#""id":"#) {`
### step_by_step_implementation_guide.md
- L161: `if text.contains(r#""id":"#) {`
- L223: `**Важное замечание:** Проверка `contains(r#""id":"#)` может найти `"id":` во вложенных объектах (ложноположительные). Для полной надежности рекомендуется использовать Envelope parsing (см. раздел "Дополнительные оптимизации").`
### step_by_step_implementation_guide_en.md
- L161: `if text.contains(r#""id":"#) {`
- L223: `**Important Note:** The check `contains(r#""id":"#)` may find `"id":` in nested objects (false positives). For full reliability, it is recommended to use Envelope parsing (see "Additional Optimizations" section).`
### benchmark_results_analysis.md
- L120: `if text.contains(r#""id":"#) {`
### benchmark_results_analysis_en.md
- L120: `if text.contains(r#""id":"#) {`
### implementation_examples.md
- L97: `if text.contains(r#""id":"#) {`
- L169: `**Важное замечание:** Проверка `contains(r#""id":"#)` может найти `"id":` во вложенных объектах. Для полной надежности рекомендуется использовать Envelope parsing (см. ниже).`
- L629: `if text.contains(r#""id":"#) {`
### implementation_examples_en.md
- L97: `if text.contains(r#""id":"#) {`
- L170: `- The check `contains(r#""id":"#)` may find `"id":` in nested objects (false positives).`
- L631: `if text.contains(r#""id":"#) {`

---

### 3.2 "Batching Re-subscriptions" Example Holds Lock Through `.await`

In `FINAL_REPORT.md` (and similar documents) the example:

- takes `subscriptions.lock().await`
- takes `subs.keys()...`
- and does `ws.send(...).await?` **without releasing lock**

This is not just a micro-issue — this is exactly the "anti-pattern" that the audit itself tries to eliminate (lock in hot path + await under lock).

The correct template in examples should be:

1) Under lock collect **owned snapshot** of channels: `Vec<String>`  
2) Drop lock  
3) Do `await send`

---

### 3.3 "Envelope Parsing" in Examples Still Uses Incorrect Pre-check for `id`

Even where `Envelope` is proposed, the pre-check remains on `"id":"` (string id). Need to change to `"id":`.

---

## 4) What's Good in Audit Overall (What I Agree With)

- Focus on CPU/alloc bottlenecks: JSON parsing + mutex contention — these are really the main costs in `ws_client.rs`.
- Idea "first cheap pre-check, then parse" — correct (but markers must match real payloads).
- Idea of batching for resubscribe — correct (but code example should avoid await under lock).

---

## 5) What I Would Add to Audit (As Report Improvement)

1) **Clearly separate**:  
   - "changes that require changing public API / architecture" (not allowed),  
   - and "changes that are local to `src/`" (allowed).  

   Currently reports have suggestions (e.g., widespread transition to DashMap), which may require changing signatures or ownership model. If the goal is *only local fixes to src*, need to explicitly mark "allowed/not allowed".

2) In the benchmarks section add a note: some tests model **artificially high contention**, which may differ from the real model (one reader-loop task). This doesn't cancel the problem, but more correctly calibrates expectations.

3) Bring code examples to "drop lock before send" in all branches:
   - RPC: `remove(id)` under lock → drop lock → `tx.send(...)`
   - subs: clone sender under lock → drop lock → send → on error remove.

---

## 6) Mini Patch-List for Documents (What Cursor Should Fix)

1) In all audit files replace marker `"id":"` with `"id":` (better as `text.contains("\"id\":")` to avoid raw-string/markdown confusion).
2) Everywhere there's `ws.send(...).await` inside a block with `subscriptions.lock().await`, rewrite example to snapshot approach.
3) Everywhere `sender.send(text)` is called under lock — rewrite example to `clone sender` → send outside lock.
4) In `optimization_recommendations(.en)` and `FINAL_REPORT(.en)` bring "Solution A / B" to one style (currently they partially contradict).

---

## 7) Output Artifact

This report is intended as a "diff-review" of Cursor's changes in `audit/`.  
If you provide the next archive, where Cursor actually fixes audit docs according to the points above, I will repeat the check and mark "closed / not closed" for each file.

