## Phase 1: Spec foundation

- [x] [serial] Create the focused OpenSpec package for generalized runtime application receipts.

## Phase 2: Inventory and schema design

- [ ] [serial] Inventory dogfood, CI, job, runtime-host, and service-start receipt/evidence surfaces.
- [ ] [depends:receipt-inventory] Define the canonical Rust receipt type or shared trait boundary and generated Nickel contract plan.

## Phase 3: Implementation

- [ ] [depends:schema-design] Add canonical serialization and validation for runtime application receipts.
- [ ] [depends:serialization] Add CLI/API readback for list/show/diagnose or document the selected readback seam.
- [ ] [depends:readback] Add redaction tests, bounded-output tests, and compatibility tests for existing dogfood/CI receipts.

## Phase 4: Documentation and validation

- [ ] [depends:tests] Update operator documentation with readback examples and receipt boundaries.
- [ ] [depends:docs] Run generated-contract freshness checks, focused receipt tests, strict OpenSpec validation, and `git diff --check`.
