# Design: signed-release-promotion-receipt

## Overview

After `molten dogfood release-promote` writes `release-promotion-gate.preserves`, the Nix dogfood check signs that receipt with:

- signer `local-release-signer`;
- trust root `local-release-trust-root`;
- key `local-release-key`;
- purpose `release-promotion`.

The check then runs `molten receipts verify-signed` with `--key-ledger "$out/signed-keyring"`, `--key-id local-release-key-v1`, and the same signer/trust-root/purpose. The verification log is stored beside the signed envelope.

## Evidence Boundary

The signature proves that the local fixture key can sign the final promotion decision and that the keyring currentness path accepts it. It does not publish a release, grant subsystem trust, or replace source, Octet, Cairn, Nix, bundle, keyring, retention, provenance, transport, policy, authority, resource, or destructive-operation gates.
