## Why

VM-CI debugging currently depends on the expensive full dogfood rail for too many distinct failure modes. Recent work proved separate shell, guest Nix, and guest Cargo smoke rails are much faster, but they are still implemented as ad-hoc dogfood commands and the full run can still appear stuck with only coarse logs, for example after Forge repository creation during source push/archive/trigger handling.

We need a first-class layered VMCI harness that localizes failures by boundary, emits structured receipts, and reserves the full CI rail for final acceptance.

## What Changes

- **Layered VMCI rails**: Promote shell/source, guest Nix, guest Cargo, source blob/materialization, and full CI checks into named harness rails with consistent invocation.
- **Boundary receipts**: Emit structured JSON evidence for host health, direct route/ticket availability, Forge source push/archive, VM registration, job assignment, workspace materialization, executor command progress, and job result publication.
- **Fail-fast phase timeouts**: Apply short, phase-specific timeouts and classify stalls at the last observed boundary instead of waiting for broad dogfood or CI timeouts.
- **Diagnostics integration**: Reuse and extend existing VMCI diagnostic summaries so failed runs preserve redacted artifact paths and machine-readable failure classes.

## Capabilities

### New Capabilities

- `dogfood-evidence.vmci.layered-harness`: Operators can run bounded VMCI rails independently and compare structured receipts across layers.
- `dogfood-evidence.vmci.phase-receipts`: Dogfood records machine-readable phase evidence and last-boundary classification for VMCI runs.

### Modified Capabilities

- `dogfood-evidence.vmci.workspace-blob-progress`: Existing post-registration diagnostics include earlier dogfood phases such as Forge source push/archive/trigger stalls.

## Impact

- **Files**: likely `crates/aspen-dogfood/src/main.rs`, `crates/aspen-dogfood/src/ci.rs`, `crates/aspen-dogfood/src/forge.rs`, `crates/aspen-dogfood/src/vmci_diagnostics.rs`, `flake.nix`, and tests.
- **APIs**: local CLI/app surface for VMCI harness rails; receipt JSON schema extension only for dogfood evidence artifacts.
- **Dependencies**: no new external runtime dependency expected.
- **Testing**: focused unit tests for rail selection, receipt classification, phase timeout behavior, and Nix app eval; live validation with shell, nix, cargo smoke rails before full VMCI acceptance.

## Out of Scope

- Making full clippy/build CI faster by itself.
- Replacing Aspen CI orchestration or worker scheduling.
- Adding HTTP endpoints or non-Iroh VM communication.
- Exposing secrets, tickets, or raw command environments in receipts.
