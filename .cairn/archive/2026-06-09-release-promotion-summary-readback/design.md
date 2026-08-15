# Design: release-promotion-summary-readback

`release-promotion-summary-v1` is a compact Preserves artifact emitted from a realized dogfood output. It reads:

- `release-promotion-gate.preserves`;
- `release-promotion-gate.signed.preserves`;
- the signed receipt keyring ledger.

The summary records the promotion decision/ref, bundle verify ref, bundle ref, signed envelope ref, signed subject ref, signed key ref, source/Octet/Cairn refs, diagnostics, and explicit checks. It passes only when the promotion receipt passes, its output path ref matches the realized output, and the signed promotion envelope verifies through the selected current key with subject ref equal to the promotion receipt ref.

The command writes summaries for both pass and deny outcomes so readback failures are themselves reviewable evidence.
