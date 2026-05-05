# Typed Nickel Contract Boundaries Verification

## Task Coverage

- I1-I3: registry, source-of-truth classification, Crunch prior-art classification, and non-candidates.
  - Evidence: `evidence/registry-slice-verification.md`, `schemas/typed-nickel-contract-registry.ncl`, `docs/typed-nickel-contract-registry.md`.
- I4-I6: Rust-derived generation, dogfood/CI receipts, deploy protocol DTOs, and freshness checks.
  - Evidence: `evidence/rust-derived-receipt-contracts-verification.md`, `evidence/deploy-protocol-contract-verification.md`, `scripts/generate-typed-nickel-contracts.py`.
- I7-I11: Nickel-authored CI/deploy, node/profile, test/patchbay, snix/trust, and crate-extraction contracts.
  - Evidence: `evidence/nickel-authored-contracts-verification.md`, `scripts/check-typed-nickel-contract-fixtures.py`.
- V1-V5: OpenSpec strict validation, Nickel checks, generator freshness/negative mutation coverage, focused Rust schema tests, and diff checks.
  - Evidence: `evidence/nickel-authored-contracts-verification.md`, `evidence/crate-extraction-readiness.md`, `evidence/crate-extraction-readiness.json`.
