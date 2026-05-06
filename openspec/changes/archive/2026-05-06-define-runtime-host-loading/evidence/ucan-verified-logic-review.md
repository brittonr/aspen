# UCAN and verified-logic review notes

Status: captured
Verification-IDs: r[runtime-host-loading.ucan-delegation.reference-reviewed], r[runtime-host-loading.ucan-delegation.boundary-preserved], r[runtime-host-loading.verified-admission.reference-reviewed], r[runtime-host-loading.verified-admission.structural-selected], r[runtime-host-loading.verified-admission.boundary-not-overclaimed]

## UCAN mapping
- Reference path inspected: `/home/brittonr/git/ucan`.
- Runtime capability bindings map to UCAN-shaped `ability`, `resource`, `proof_refs`, and typed caveat value-shape fields.
- `aspen-runtime-core` intentionally does not depend on UCAN shell behavior; cryptographic verification, proof resolution, revocation backend traversal, and policy I/O remain runtime/shell boundaries.

## Verified-logic mapping
- Reference path inspected: `/home/brittonr/git/verified-logic` plus UCAN-vendored verified predicates.
- Candidate finite predicates: host-kind/artifact compatibility, artifact hash string shape, OCI digest shape, resource-bound shape, ability/resource syntax, proof-hop-depth bound, and typed caveat payload shape.
- Implemented first structural admission seam in pure Rust (`admit_unit`, `admit_receipt`) but did not overclaim formal verification yet.
- Explicit non-verified boundaries: cryptographic signature strength, sandbox implementation, scheduler fairness, network resolution, filesystem materialization, and external policy backends.
