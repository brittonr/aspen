## Why

The deterministic distributed simulation layer is passing and already covers the core evidence contract, but branch-specific evidence is still uneven. Several fault classes are implemented in the simulator without direct named fixtures, and distributed CI gate tests still lean on synthetic metadata. That leaves release reviewers with less precise evidence when a future regression hits stale evidence, corrupted receipts, resource pressure, crash, rejoin, drop, reorder, or profile wiring.

## What Changes

- Add direct positive fixtures for benign simulation faults: delay, drop, reorder, rejoin, crash, restart, and duplicate suppression where applicable.
- Add direct negative fixtures for stale evidence, corrupted receipts, resource pressure, unauthorized transport authority, ambient-state drift, and partitioned quorum denial.
- Assert each fixture's decision, committed or denied operation set, event kind, diagnostic, receipt ref stability, and final-state ref stability.
- Strengthen distributed CI profile wiring tests so profile ids, command surfaces, expected artifact kinds, retry policy, unavailable handling, and variance declarations come from the configured matrix instead of ad hoc-only metadata.
- Keep VM and live soak claims out of simulation receipts; this change only strengthens fast deterministic review evidence.

## Impact

This closes the direct fixture gaps in the fast simulation layer and gives traceability/release review clearer evidence for every supported fault branch. It should not change public runtime behavior, transport behavior, authority semantics, or VM/live evidence requirements.
