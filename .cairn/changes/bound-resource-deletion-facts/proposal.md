# Proposal: Bound resource deletion-gate facts

## Why

`resources::evaluate_deletion_gate` is a public API for owner, finalizer, pin, retention, and authority facts. Unlike resource metadata validation, it previously accepted arbitrarily many caller-supplied facts and arbitrarily long strings, then formatted an output row for each blocker or clearance and cloned the resulting diagnostics. The bound Octet catalog marks six distinct append positions in this producer (F2678–F2689, each reported twice). A compiled external caller demonstrated that 257 pins and a 4097-byte fact both produced ordinary `blocked` decisions rather than failing admission.

## What Changes

- Admit at most 64 owner refs and 64 finalizers, reusing resource metadata limits. Admit at most 256 cleanup receipts, live owner refs, pins, retention refs, and deletion authority refs, reusing the evidence-ref limit. r[molten.lifecycle.gc_cleanup_gates.bounded_facts]
- Admit fact strings of at most 4096 UTF-8 bytes, the existing resource annotation-value limit. Inspect all supplied counts and fact lengths before formatting or cloning any diagnostic.
- Preserve exact-bound `blocked` decisions for live pins and owners and `deletion-ready` decisions after blockers clear. An out-of-bound input returns an error, not a deletion decision or authority grant.

## Impact

- **Producer and tests**: `src/resources/parts/mod/p004/body.rs`, `src/resources/parts/mod/p003/body.rs`.
- **Contract**: a separate change-local `node-runtime` delta; no accepted specification, pin, waiver, commit, or archive is modified.
- **Caller**: the public Rust API has no product CLI callsite. Its behavior is exercised by a disposable compiled external consumer, not an unrelated traceability CLI command.
