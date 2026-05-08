## Phase 1: Redaction Inventory

- [x] [serial] Create the OpenSpec baseline for operator receipt redaction hardening.
- [x] [serial] Inventory dogfood receipt list/show/diagnose and runtime-host evidence rendering paths that can display receipt or failure data.

## Phase 2: Implementation

- [x] [depends:inventory] Extract or identify pure render/diagnose helpers for the targeted operator-visible receipt output.
- [x] [depends:helpers] Add positive assertions for useful non-secret evidence and negative assertions for injected secret markers.
- [x] [depends:negative-tests] Add or update docs/evidence guidance describing the redaction boundary.

## Phase 3: Verification

- [x] [depends:docs] Run focused receipt redaction tests, relevant CLI/render tests, OpenSpec validation, and whitespace checks.
