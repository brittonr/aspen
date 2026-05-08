## Why

Runtime-host readiness now spans docs, Nickel suite manifests, generated inventory, OpenSpec records, proof markers, and anti-overclaiming boundaries. A focused acceptance bundle check would prevent those surfaces from drifting after the recent row promotions.

## What Changes

- Add a deterministic runtime-host acceptance bundle check.
- Assert promoted product-path rows, marker anchors, manifest names, docs, and non-proof boundaries stay synchronized.
- Keep the check local and bounded; it does not execute gated KVM/Uhyve/Hyperlight proofs by default.

## Capabilities

### Modified Capabilities
- `runtime-host-loading`: Adds acceptance-bundle consistency requirements for promoted runtime-host evidence.

## Impact

- **Files**: runtime-host docs/tests, harness inventory checks, and possibly Nix check wiring.
- **APIs**: Internal test/check surface only.
- **Testing**: focused acceptance-bundle check, `scripts/test-harness.sh check`, runtime-host docs guard, OpenSpec validation, whitespace checks.
