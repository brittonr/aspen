## Why

The VM-CI dogfood recovery work has moved past the original guest-to-host Iroh/QUIC blocker: guest workers can receive `ci_nix_build` over the bridge. The remaining failure mode is a long-running or stalled VM CI job around workspace/blob materialization, guest executor progress, or job-result publication. Today this requires ad-hoc log greps and manual process cleanup, which makes it too easy to rerun the full dogfood loop without learning where the stall moved.

## What Changes

- **Post-connection VM-CI diagnostics**: Add deterministic diagnostics for the boundary after VM worker registration and job assignment.
- **Workspace/blob progress evidence**: Require structured evidence showing whether the VM guest fetched workspace blobs, started the executor, invoked Nix, produced logs, or timed out before each step.
- **Bounded stall classification**: Distinguish bridge/firewall regressions from post-registration CI workspace/blob/job execution stalls.
- **Evidence preservation**: Preserve redacted host/guest logs and run receipts before cleanup so the OpenSpec can be archived with durable proof.

## Capabilities

### Modified Capabilities
- `dogfood-evidence`: Adds VM-CI post-registration diagnostic evidence and classification requirements for dogfood receipts/logs.

## Impact

- **Files**: Likely `crates/aspen-dogfood`, `crates/aspen-ci-executor-vm`, CI job/workspace/blob handling code, and dogfood scripts/log preservation paths.
- **APIs**: No public API is required unless existing receipt/log commands cannot expose the evidence.
- **Dependencies**: No new external dependency expected.
- **Testing**: Focused unit tests for classifier/receipt logic, VM executor tests for workspace/blob error reporting, strict OpenSpec validation, and a live VM-CI retry that reaches a classified result instead of an unbounded stall.

## Out of Scope

- Reworking the already-landed bridge, TAP helper, firewall-chain, or relay-disabled ticket scoping unless diagnostics prove a regression there.
- Solving unrelated Nix closure build timeouts before the dogfood node starts.
- Treating successful worker registration as full VM-CI acceptance without a completed CI job receipt.
