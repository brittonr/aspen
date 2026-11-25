# Evidence gates deltas

## ADDED Requirements

### Requirement: Evidence chains, provenance, traceability derivation, and hardening receipts live in a no_std kernel
r[molten.evidence_gates.no_std_evidence_proof_kernel] Evidence chain validation, provenance lineage, aggregate/layered proof composition, verification-run traceability derivation, drift comparison, and testing-hardening receipt builders MUST live in a `#![no_std]` + `alloc` crate (`molten-evidence-proof-core`) built atop `molten-preserves-core` without `std::path::PathBuf` or filesystem I/O.

#### Scenario: Proof and traceability derivation execute under no_std
- GIVEN canonical packed verification-run receipts and an aggregate proof manifest
- WHEN `molten-evidence-proof-core` derives coverage and validates composition under `#![no_std]`
- THEN it MUST emit deterministic canonical receipts and diagnostics without reading files or environment variables.
