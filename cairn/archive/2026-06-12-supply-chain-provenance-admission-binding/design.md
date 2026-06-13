## Context

The `supply-chain-provenance-builds` slice added canonical build records and build verification receipts. The existing provenance evaluator admitted `reproducible-verified` as a trust state, but it did not require the evaluator or node-control gates to receive a matching build verification receipt.

## Goals

- Treat `reproducible-verified` as an evidence-bound trust state, not a self-asserted string.
- Bind provenance records to explicit build record refs through a canonical `build-records` field.
- Bind provenance evaluation receipts to the build verification receipt refs considered during evaluation.
- Fail closed when build verification evidence is missing, denied, names a different expected/actual artifact, or references a build record not named by the provenance record.
- Preserve reviewed and policy-trusted provenance behavior without requiring build verification receipts.
- Preserve the evidence-only boundary: build verification satisfies only provenance evidence and never grants authority, policy, resource, transport, execution, or source-gate trust.

## Non-Goals

- No real build execution or rebuild orchestration.
- No authority, policy, resource, transport, execution, or source-gate admission from build verification receipts.
- No global trust upgrade from content hashes or Nix derivation refs alone.
- No remote-sync transport changes in this slice beyond keeping provenance evidence shapes compatible with node-control gates.

## CLI Shape

```sh
molten test provenance record \
  --artifact-ref blake3:artifact \
  --trust-state reproducible-verified \
  --source-ref blake3:source \
  --dependency-closure-ref blake3:deps \
  --toolchain-ref blake3:toolchain \
  --builder-ref blake3:builder \
  --build-record-ref blake3:build-record \
  --out target/provenance.reproducible.preserves

molten test provenance evaluate \
  --operation install \
  --profile node-control \
  --artifact-ref blake3:artifact \
  --provenance target/provenance.reproducible.preserves \
  --build-verification target/provenance.build-verify.preserves \
  --receipt-out target/provenance.receipt.preserves
```

## Evidence Boundary

`provenance-receipt-v1` now records the build verification receipt refs supplied to the evaluator. These refs explain why a reproducible trust state was admitted or denied. They remain diagnostic/provenance evidence only and callers must still provide independent authority, policy, resource, transport, execution, and source-gate evidence.
