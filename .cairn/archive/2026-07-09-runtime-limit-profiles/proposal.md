## Why

Molten has many named Rust hard caps for safety, but operators still need reviewed configuration for practical runtime budgets: control-loop bounds, live-send attempts/timeouts, frame limits, chunk sizing, replay/harness budgets, and retention scan windows. Today those values are scattered between CLI defaults, Rust constants, Nix checks, and docs.

A runtime limit profile can make tunable budgets explicit while preserving fail-closed hard caps in Rust.

## What Changes

- Define runtime limit profiles for operator-selected budgets under compiled hard caps.
- Validate selected limits through a pure admission core before node startup, service loops, live transport, chunk/storage operations, retention GC, and harness runs use them.
- Bind admitted effective limits into relevant receipts and effective-config readbacks.
- Keep hard caps compiled and named; profiles can only select values within those caps unless a separate reviewed code change raises the cap.
- Add positive and negative tests for valid profile values, one-past-hard-cap denials, incoherent limits, default caveats, and receipt binding.

## Impact

- **Files**: resource governance core, node/service/live/chunk/retention/harness limit inputs, profile contracts, receipts, docs, and tests.
- **Testing**: pure admission tests for bounds and coherence plus integration tests for representative runtime surfaces.
- **Safety**: limit profiles constrain resource use. They do not grant data authority, policy admission, source-gate trust, provenance trust, retention clearance, transport authority, execution permission, or release eligibility.
