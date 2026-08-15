## Why

Peer sessions already record lifecycle state, bootstrap refs, capability refs, authority refs, and diagnostics, but the semantic transition relation is not first-class. That makes it too easy for future code to accept an impossible state jump when the local evidence checks happen to pass.

Molten needs peer session transitions to be a reviewed finite relation with explicit events, guards, terminal-state behavior, and receipt evidence.

## What Changes

- Model peer sessions with a closed transition relation over prior state, requested event, target state, and explicit guard facts.
- Deny invalid skips, terminal-state exits, quarantine bypasses, wrong-topic evidence, stale tickets, revoked evidence, and missing admissions without advancing state.
- Bind transition receipts to before-state, event, target, after-state or preserved-state refs, guard evidence refs, decision, and diagnostics.
- Add positive and negative fixtures plus generated transition traces for valid peer progression and invalid transport-only/state-skip attempts.

## Impact

Peer-session code becomes a small deterministic FSM core with a boring shell. Live transport observations, handoff imports, and connected sessions remain evidence-only until the explicit transition and normal authority/capability gates pass.