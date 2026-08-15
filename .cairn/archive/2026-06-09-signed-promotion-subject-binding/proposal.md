# signed-promotion-subject-binding

## Summary

Require the Nix dogfood signed promotion receipt verification to bind the exact release promotion gate receipt subject ref.

## Motivation

The signed promotion receipt already proves the final promotion decision is signable and keyring-verifiable. The dogfood check should also fail closed if the signed envelope points at any subject other than the promotion receipt that was just emitted.

## Scope

- Extract the promotion receipt ref from the `release-promote` CLI status line.
- Pass that ref as `--subject-ref` to `molten receipts verify-signed` for `release-promotion-gate.signed.preserves`.
- Document the subject-ref binding.

## Non-Goals

- Changing signed receipt cryptography.
- Granting release publication authority.
- Replacing bundle, keyring, source, Octet, Cairn, Nix, or subsystem gates.
