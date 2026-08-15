# Design: vat generic replay receipts

## Integration point

`molten test vat replay-fixture` continues to emit vat-local replay runs and `vat-replay-receipt-v1` records. This slice adds generic deterministic replay evidence to the same fixture output by invoking the generic deterministic replay fixture verifier from the vat fixture construction path.

The fixture should include:

- a passing `deterministic-replay-verify-v1` receipt for the unchanged generic fixture;
- a denial `deterministic-replay-verify-v1` receipt for a changed effect response or equivalent first-divergence case;
- the corresponding `deterministic-first-divergence-v1` value when the generic verifier emits one;
- diagnostics that expose the generic receipt refs alongside vat-local receipt refs.

## Boundaries

The integration is evidence-only. A matching generic replay receipt does not grant object authority, debugging authority, transport trust, persistence trust, policy admission, provenance trust, resource authority, or source-gate trust. Vat-specific authority and predicate evidence remains required.

## Compatibility

Existing vat-local records remain in place so current tests and tooling keep working. The generic receipts are additional evidence and should be canonical Preserves values embedded in the vat replay fixture output.
