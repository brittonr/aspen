## Context

Molten's `delivery_idempotency` module provides canonical operation ids, scoped delivery windows, Redb-backed dedup entries, idempotency receipts, and retry receipts. Remote dataspace and node-control ingress call this module before committing side effects, but the CLI did not expose the same primitives directly.

## Goals

- Expose canonical delivery scope profile records and refs.
- Expose canonical operation id records for explicit scope/profile/name, producer, consumer, sequence, intent, payload ref, and policy refs.
- Run a delivery idempotency check against an explicit store root and emit the same receipt shape as production call sites.
- Display delivery operation ids, windows, dedup entries, idempotency receipts, and retry receipts from files or the store.
- Preserve fail-closed validation in the module: malformed refs, unsupported scope profiles, unsupported gap policy values, and unknown receipt refs remain errors.

## Non-Goals

- No new delivery semantics beyond the existing scoped idempotency state machine.
- No network-level exactly-once claim.
- No authority/provenance/policy grant; refs are evidence inputs only.
- No mutation of remote dataspace or node-control queues from the delivery CLI itself.

## CLI Shape

```sh
molten test delivery scope \
  --scope-profile remote-dataspace-topic \
  --scope-name peer:b:services \
  --retention-ref blake3:policy \
  --out target/delivery.scope.preserves

molten test delivery operation-id \
  --scope-profile remote-dataspace-topic \
  --scope-name peer:b:services \
  --producer peer:a/producer --consumer peer:b --sequence 1 \
  --intent remote-dataspace-assert --payload-ref blake3:payload \
  --policy-ref blake3:policy --out target/delivery.operation.preserves

molten test delivery check \
  --root target/delivery-store \
  --scope-profile remote-dataspace-topic \
  --scope-name peer:b:services \
  --producer peer:a/producer --consumer peer:b --sequence 1 \
  --intent remote-dataspace-assert --payload-ref blake3:payload \
  --policy-ref blake3:policy --evidence-ref blake3:evidence \
  --semantic-result-ref blake3:result \
  --receipt-out target/delivery.first.preserves
```

`check` returns `first` with side effect `commit` for the first admitted sequence, `duplicate` with side effect `suppress` for exact duplicate evidence, `conflict` for reused sequence with different payload/evidence, and `gap`/`retry` for future sequences according to the chosen gap policy.

## Evidence Boundary

All command outputs remain canonical Preserves artifacts. Text summaries are non-normative diagnostics over those artifacts. The delivery CLI reads or writes only the explicit `--root` store and explicit output paths.
