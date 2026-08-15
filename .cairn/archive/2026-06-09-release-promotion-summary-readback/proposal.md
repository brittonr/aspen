# release-promotion-summary-readback

## Summary

Add an operator readback summary for release promotion evidence that binds the promotion receipt, signed promotion envelope, selected keyring key, and source/Octet/Cairn refs.

## Motivation

Dogfood release outputs now contain the release bundle, promotion gate, signed promotion envelope, and signed keyring. Operators need one compact artifact and CLI status line that summarizes whether the complete promotion readback is coherent without treating logs as normative evidence.

## Scope

- Add canonical `release-promotion-summary-v1` evidence.
- Add `molten dogfood release-promotion-summary`.
- Verify `release-promotion-gate.signed.preserves` through the signed keyring and bind it to the promotion receipt subject ref.
- Emit deny summaries for missing promotion receipts, missing signed promotion envelopes, subject mismatches, revoked/stale keys, or output path mismatches.
- Include the summary in the Nix dogfood output.

## Non-Goals

- Publishing releases.
- Granting authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation, or release publication trust.
