## Why

Many modules duplicate bounded collection helpers and diagnostic sinks. The duplicates differ in overflow handling, limit diagnostics, method names, and whether arithmetic is checked before comparing bounds. This makes denial behavior harder to audit and invites drift at resource-sensitive boundaries.

## What Changes

- Extend the shared bounded utility module with checked count arithmetic, bounded push/extend helpers, and diagnostic sink adapters.
- Replace local `PushLimited`, `DiagnosticSink`, `BoundedPush`, `ensure_count_at_most`, and `push_bounded` duplicates where behavior is equivalent.
- Preserve existing public receipt shapes while normalizing overflow and bound-denial diagnostics.

## Impact

- **Files**: `src/bounded/mod.rs` plus migrated parser, diagnostic, plugin, node, testing, coordination, retention, and adapter call sites.
- **Testing**: helper-level positive/negative tests cover exact limit, limit overflow, arithmetic overflow, extend overflow, and diagnostic sink behavior.
