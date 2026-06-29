## Phase 1: Gateway decision core

- [x] [serial] r[molten.operator_gateway.readback_core] Add pure gateway request, range, visibility, and readback decision types.
- [x] [serial] r[molten.operator_gateway.visibility_retention] Wire retention, confidentiality, redaction, and reveal-gate inputs into gateway read/index decisions.

## Phase 2: Verified range readback

- [x] [serial] r[molten.operator_gateway.verified_range_read] Map requested byte ranges to chunk-store manifest ranges and verify relevant chunks before exposing bytes.
- [x] [serial] r[molten.operator_gateway.verified_range_read] Add positive and negative tests for full reads, partial reads, corrupt chunks, wrong lengths, unsupported transforms, invalid ranges, and range overflows.

## Phase 3: Index and MIME hints

- [x] [serial] r[molten.operator_gateway.readonly_index] Add read-only index rendering decisions for artifact bundles, chunk collections, release evidence bundles, and retention bundles.
- [x] [serial] r[molten.operator_gateway.readonly_index] Ensure hidden refs, sensitive names, and denied members are omitted or redacted according to policy.

## Phase 4: Receipts and shell

- [x] [serial] r[molten.operator_gateway.receipts] Emit canonical gateway read, range, and index receipts binding requests, decisions, refs, ranges, policy evidence, and diagnostics.
- [x] [serial] r[molten.operator_gateway.receipts] Add a CLI fixture or minimal read-only HTTP shell that uses the pure decision core before response streaming.

## Phase 5: Documentation and validation

- [x] [serial] r[molten.operator_gateway.receipts] Document the read-only/evidence-only trust boundary and the `iroh-gateway` reference pattern.
- [x] [serial] r[molten.operator_gateway.receipts] Run focused chunk-store/catalog/gateway tests and Cairn validation.
