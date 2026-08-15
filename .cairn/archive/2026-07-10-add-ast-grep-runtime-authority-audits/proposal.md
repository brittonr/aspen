## Why

Aspen/Molten is a policy-gated distributed runtime with explicit authority, effect, replay, sealed-repro, and evidence boundaries. ast-grep can quickly inventory syntax-level authority leaks, ambient host effects, direct network/process/filesystem calls, unsafe hotspots, and migration candidates in runtime code. Aspen should use ast-grep as a structural audit input without weakening its Preserves, Basalt/UCAN, Cairn, Octet, Valence, and replay evidence boundaries.

## What Changes

- Define ast-grep runtime-authority audit profiles for core runtime, node control, effect handlers, plugin host, sealed repro, Iroh transport, and policy/evidence gates.
- Require positive and negative fixtures before any audit rule can become blocking.
- Record ast-grep tool identity, rule bundle BLAKE3 hash, scan scope, findings, and non-claims in runtime/evidence-gate receipts.
- Keep ast-grep findings as candidate structural evidence, not runtime authority or replay correctness proof.

## Impact

- **Surfaces**: evidence gates, runtime spine, node runtime, plugin host, resource governance, confidentiality, operator workflow.
- **Non-claims**: ast-grep does not prove deterministic replay, capability admission, UCAN authority, sealed-repro correctness, distributed safety, or release readiness.
- **Validation**: ast-grep rule tests, authority-boundary fixture scans, Cairn gates, and focused Aspen/Molten validation rails.
