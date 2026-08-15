# signed-release-promotion-receipt

## Summary

Sign and keyring-verify the final release promotion gate receipt emitted by the Nix dogfood release check.

## Motivation

The release promotion gate aggregates the realized release evidence graph into one pass/deny receipt. The dogfood check should also prove that this final review decision can be wrapped in the same ledger-backed signed receipt/keyring flow used for bundle members, without treating the signature as release authority.

## Scope

- Emit `release-promotion-gate.signed.preserves` in `dogfood-local-node`.
- Verify the signed promotion receipt through the local signed receipt keyring.
- Use a distinct signing purpose, `release-promotion`, so promotion decisions do not share the `release-evidence` member-purpose namespace.
- Document that the signed promotion receipt remains evidence-only.

## Non-Goals

- Changing release publication authority.
- Adding new cryptographic primitives.
- Making promotion signatures a substitute for source, Octet, Cairn, Nix, keyring, bundle, or subsystem gates.
