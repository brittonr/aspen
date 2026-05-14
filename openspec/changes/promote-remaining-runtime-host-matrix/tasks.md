## Phase 1: Spec foundation

- [x] [serial] Create the focused OpenSpec package for remaining runtime-host metadata-row promotion.

## Phase 2: Matrix inventory and candidate selection

- [ ] [serial] Audit runtime-host readiness docs, harness manifests, and runtime-host-loading specs for rows that remain metadata-only or future-work.
- [ ] [depends:matrix-audit] Select the next row by ROI and write a row-specific implementation plan before code changes.

## Phase 3: Product-path promotion

- [ ] [depends:row-plan] Add or identify the product orchestration seam that submits or reconciles the selected host kind without direct helper-only execution.
- [ ] [depends:product-seam] Add deterministic fixture artifacts and proof markers for the selected host kind.
- [ ] [depends:fixture] Add positive product-path proof and negative overclaiming guardrails.
- [ ] [depends:proof] Update harness metadata, generated inventory, and runtime-host readiness docs after the proof passes.

## Phase 4: Validation

- [ ] [depends:docs-harness] Run focused row proof, `scripts/test-harness.sh check`, strict OpenSpec validation, and `git diff --check`.
