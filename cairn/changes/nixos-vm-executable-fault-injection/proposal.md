## Why

The current NixOS VM test proves a two-node platform path and records a fault matrix, but several fault cases are still simulated as diagnostic evidence. To strengthen distributed testing, VM checks should execute representative platform-level network, process, storage, and restart faults when host support is available, while failing or marking unavailable rather than minting pass evidence when it is not.

## What Changes

- Add executable VM fault injection for network delay/drop/partition/rejoin using platform tools available inside the NixOS test driver.
- Exercise crash and restart windows around queued control work, partial dispatch, receipt writes, and duplicate sends.
- Add storage and state-root failure cases such as permission denial, missing artifacts, and bounded disk pressure where deterministic VM support permits.
- Emit canonical VM fault receipts that bind injection commands, pre/post evidence, child refs, decisions, diagnostics, host-support status, and evidence-only caveats.
- Keep unsupported host/KVM/network features fail-closed or unavailable; never convert skipped executable faults into pass evidence.

## Impact

This improves confidence in the VM layer without claiming broad WAN, fleet-scale, or production readiness. Simulated fault matrix evidence remains useful, but executable VM fault evidence becomes the preferred proof for platform-integration claims.
