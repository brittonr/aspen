# Design: provenance trust state proof

## Scope

This change proves provenance trust-state admission. It covers reviewed records, reproducible build records, build verification receipts, policy-trusted records, denied/missing records, operation profiles, sensitive operation thresholds, and catalog/readback classification.

## Proof checklist

- **Proof claim**: each operation admits only the minimum required provenance trust state for its profile, and reproducible-verified trust is accepted only when a passing build verification receipt binds the same artifact and build record.
- **Out of scope**: build system correctness beyond canonical build records and verification receipts.
- **Trusted assumptions**: source-gate, Octet, builder, and dependency-closure refs are valid when referenced by passing verification receipts.
- **Positive evidence**: reviewed low-risk admission, reproducible-verified sensitive admission with matching build verification, and policy-trusted admission when policy evidence matches.
- **Negative evidence**: missing provenance, wrong artifact, wrong profile, stale build verification, mismatched build record, denied trust state, and weak trust for sensitive operations deny.
- **Canonical refs**: provenance record ref, artifact ref, build record ref, build verification receipt ref, source/dependency/toolchain/builder refs, operation/profile ids, and diagnostics.
- **Regeneration command**: `cargo test provenance node job`.

## Functional core

Model provenance admission as a pure comparison between requested operation profile, artifact ref, provenance records, build verification receipts, and prior diagnostics. Node, job, and remote-sync shells only consume pass/deny decisions.

## Non-goals

- No authority, policy, resource, source-gate, or execution grant from provenance alone.
- No claim that human-readable build logs are normative evidence.
