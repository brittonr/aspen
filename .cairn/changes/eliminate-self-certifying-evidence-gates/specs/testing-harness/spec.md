# Testing harness deltas

## ADDED Requirements

### Requirement: Evidence matrix and capability qualification use non-placeholder native producers
r[molten.testing.evidence_matrix_native_producers] `tests/evidence-matrix.ncl` MUST NOT use sequential placeholder `blake3:000...` refs or alias one `--ignored` test across unrelated positive and negative rows. Every product capability mapped in `config/capability-report/capabilities.json` MUST bind to exact matrix requirements and, where locally executable, supply a real native execution producer rather than remaining synthetic-harness-only.

#### Scenario: Placeholder digest in evidence matrix denies validation
- GIVEN an entry in `tests/evidence-matrix.ncl` containing a sequential placeholder digest or aliasing one ignored test as both positive and negative evidence
- WHEN matrix validation runs
- THEN validation MUST fail closed with a typed diagnostic.

#### Scenario: Local product capabilities emit exact-subject native receipts
- GIVEN a locally runnable capability family (durable state recovery, effect uncertainty, cross-process transport, world replay/promotion/restore, or control-plane failover)
- WHEN `molten evidence qualify` executes its native producer
- THEN it MUST emit an exact-subject canonical Preserves receipt bound to actual runtime execution.
