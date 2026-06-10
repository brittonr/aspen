# release-promotion-gate

## Summary

Add a final release-promotion evidence gate that aggregates the release evidence bundle verification, signed keyring currentness, and source/Octet/Cairn evidence markers into one canonical pass/deny receipt.

## Motivation

Molten release dogfood now emits release bundles, signed member receipts, and ledger-backed keyring evidence. Operators still need a single review artifact that says whether the realized output is promotable under the local evidence profile without treating signatures or dogfood artifacts as authority. The promotion gate provides that artifact and keeps all subsystem gates explicit.

## Scope

- Add canonical `release-promotion-gate-receipt-v1` evidence.
- Add `molten dogfood release-promote` CLI.
- Bind release bundle verify receipt refs, selected signed keyring refs, source evidence markers, Octet evidence markers, and Cairn evidence markers.
- Emit deny receipts for failed bundle verification, output mismatch, missing evidence markers, missing/ambiguous/revoked signed keys, or stale keyring currentness.
- Integrate promotion evidence into the Nix dogfood check output.

## Non-Goals

- Publishing releases or changing tags.
- Granting authority, policy, provenance, resource, transport, source-gate, retention, or destructive-operation trust.
- Replacing Octet, Cairn, Nix, source, provenance, or subsystem gates.
