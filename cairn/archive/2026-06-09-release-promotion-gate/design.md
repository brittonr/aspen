# Design: release-promotion-gate

## Overview

`release-promotion-gate-receipt-v1` is a canonical Preserves receipt that binds:

- the `release-evidence-bundle-verify-receipt-v1` ref and its bundle/report/release-gate/Nix refs;
- the realized Nix dogfood output path ref;
- the selected current signed receipt key ref, key id, signer, trust root, generation, and observed revocation refs;
- source, Octet, and Cairn evidence markers as deterministic BLAKE3 refs;
- diagnostics and explicit checks;
- evidence-only caveats.

The promotion receipt decision is `pass` only when the bundle verification receipt passes, the output path ref still matches the realized output being promoted, source/Octet/Cairn evidence markers are non-empty, and exactly one selected key is current and unrevoked under the configured signer/trust-root/key id/ref.

## CLI

`molten dogfood release-promote` reads a bundle verify receipt and a signed receipt keyring ledger:

```sh
molten dogfood release-promote \
  --output-path OUT \
  --bundle-verify OUT/release-evidence-bundle-verify.preserves \
  --receipt-out OUT/release-promotion-gate.preserves \
  --signed-key-ledger OUT/signed-keyring \
  --signed-key-id local-release-key-v1 \
  --signed-trust-root local-release-trust-root \
  --signed-signer local-release-signer \
  --source-evidence SOURCE-MARKER \
  --octet-evidence OCTET-MARKER \
  --cairn-evidence CAIRN-MARKER
```

The command writes a receipt for both pass and deny outcomes. Denials stay inspectable through the receipt diagnostics.

## Evidence Boundary

Promotion evidence is a review summary. It does not perform release publication, grant trust, or bypass source, Octet, Cairn, Nix, retention, provenance, transport, policy, authority, resource, or destructive-operation gates.
