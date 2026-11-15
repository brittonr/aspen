# Check consensus fast-path hazard semantics

## Why
Issue #5 requires actual TLC/TLA-ts verdict and state-count parity, not parser acceptance, and a guard mutant that produces a causal counterexample. The current set-only model does not express order agreement or retain admission evidence.

## What changes
Keep the hand-written TLA+ source authoritative. Express bounded order, recovery and admission witnesses; map every profile invariant and fault schedule to a property or explicit abstraction boundary. Add a pinned, bounded dual-checker campaign and retain raw logs, summaries and mutant traces. Investigate Quint conformance and optional Veil independently without representing either as TLC/TLA-ts evidence.

## Non-goals
No live-engine, transport, durability, unbounded consensus, performance or release proof; no vendor archive fork and no invented execution results.
