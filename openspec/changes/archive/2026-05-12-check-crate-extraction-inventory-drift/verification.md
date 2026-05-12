# Verification

## Task Coverage

- Task: 1.1 Fix the checker syntax regression so the Rust script compiles.
  - Evidence: `openspec/changes/archive/2026-05-12-check-crate-extraction-inventory-drift/evidence/rustfmt.txt`, `openspec/changes/archive/2026-05-12-check-crate-extraction-inventory-drift/evidence/testing-harness-readiness.md`
- Task: 1.2 Add inventory-row parsing for selected crate-extraction families.
  - Evidence: `scripts/check-crate-extraction-readiness.rs`, `openspec/changes/archive/2026-05-12-check-crate-extraction-inventory-drift/evidence/testing-harness-readiness.md`
- Task: 1.3 Compare manifest links, owners, readiness state, and stale next-action text against typed policy.
  - Evidence: `scripts/check-crate-extraction-readiness.rs`, `openspec/changes/archive/2026-05-12-check-crate-extraction-inventory-drift/evidence/negative-stale-inventory.txt`
- Task: 2.1 Run the checker on a ready family and save evidence.
  - Evidence: `openspec/changes/archive/2026-05-12-check-crate-extraction-inventory-drift/evidence/testing-harness-readiness.md`, `openspec/changes/archive/2026-05-12-check-crate-extraction-inventory-drift/evidence/testing-harness-readiness.json`
- Task: 2.2 Prove a stale/mismatched inventory fixture fails deterministically.
  - Evidence: `openspec/changes/archive/2026-05-12-check-crate-extraction-inventory-drift/evidence/negative-stale-inventory.txt`
- Task: 2.3 Run OpenSpec validation.
  - Evidence: `openspec/changes/archive/2026-05-12-check-crate-extraction-inventory-drift/evidence/openspec-validate.txt`
