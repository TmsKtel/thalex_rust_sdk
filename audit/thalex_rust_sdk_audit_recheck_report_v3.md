# Thalex Rust SDK — Audit Folder Recheck (Post-Cursor Edits)
Date: 2025-12-17
Scope: Only documentation in `thalex_rust_sdk/audit/` was reviewed (no changes to `./src`). This recheck compares the updated audit documents against issues raised in `thalex_rust_sdk_audit_recheck_report_v2.md`.
## Executive summary
- ✅ The major correctness issues from the previous recheck were addressed in the audit docs.
- ✅ Examples now consistently treat JSON-RPC `id` as **numeric** (`"id":123`) and use `"id":` (not `"id":"`).
- ✅ The reconnection re-subscribe guidance now shows **snapshot under lock** and **send outside lock**, and includes **batched subscribe** (`params.channels = [...]`).
- ✅ The docs no longer present examples that hold `subscriptions.lock().await` across an `await`ed `ws.send(...)`.
- ⚠️ Remaining items are mostly editorial/consistency (not correctness), plus one recommendation to explicitly label which code blocks are “current” vs “optimized” to avoid confusion.
## 1) Checklist against v2 recheck findings
### 1.1 JSON-RPC `id` treated as string in fast-check examples
**Status:** ✅ Fixed in updated audit docs.
- Where the docs mention the pitfall, they now correctly state: `"id":123` not `"id":"123"`.
- Fast-check examples now use `text.contains("\"id\":")` (numeric id), not `contains(r#""id":"#)`.
### 1.2 Lock held across `.await` in re-subscribe examples
**Status:** ✅ Fixed.
- Updated examples show:
  1) snapshot keys under lock into `Vec<String>`;
  2) release lock;
  3) `ws.send(...).await` outside lock.
### 1.3 Subscription batching guidance
**Status:** ✅ Present and consistent.
- Updated examples show a single `public/subscribe` with `params: { channels: [...] }`.
## 2) What I verified concretely
### 2.1 No remaining `"id":"..."` checks in code blocks
I scanned fenced code blocks across audit markdown files for these specific anti-patterns:
- `r#""id":"#`
- `"id":"` (as the check key)

**Result:** No code blocks use these patterns anymore.
(The string form `"id":"123"` still appears, but only as an explanatory *counterexample*, which is correct.)
### 2.2 No examples that lock subscriptions and then `await` a send
I scanned fenced code blocks for patterns like:
- `let subs = subscriptions.lock().await; ... ws.send(...).await`

**Result:** No matches found in the updated audit docs.
## 3) Minor remaining improvements (editorial, not correctness)
1) **Make “current vs optimized” labeling consistent**
- Some examples mix both in one block (which is fine), but it’s easy for a reader to copy the wrong half.
- Recommendation: prefix with `// CURRENT (existing src)` and `// OPTIMIZED (proposed)` on every paired snippet.

2) **Avoid ellipses `...` inside code blocks when the snippet is intended to compile**
- A few blocks end with `...` after `ws.send(...)`.
- Recommendation: either remove `...` or explicitly mark the snippet as pseudocode.

3) **Optional: add a short “Assumptions” paragraph about server-side support for channel batching**
- The docs assume `public/subscribe` accepts an array of channels.
- If that’s guaranteed by Thalex API, add a one-liner citing the relevant API behavior (no link needed if internal).
## 4) Bottom line
From a documentation/audit-report standpoint, the previously identified correctness problems appear resolved. The audit folder now gives consistent, technically correct guidance under the stated constraints (optimize `src/` only, no SDK refactor).
