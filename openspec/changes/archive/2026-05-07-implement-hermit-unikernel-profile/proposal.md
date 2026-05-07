## Why

HermitOS-style unikernels are guest artifacts under a VM/microVM host boundary. Aspen needs a focused profile for Uhyve and loader/QEMU launch paths so Hermit does not get confused with OCI or native process execution.

## What Changes

- Define `Unikernel { HermitOs }` artifact identity and launch profile requirements.
- Specify Uhyve and loader/QEMU runner compatibility.
- Require boot argument, serial log, and receipt redaction policy.

## In Scope

- Active OpenSpec package for the Hermit unikernel profile implementation seam.
- Requirements, design constraints, implementation tasks, and verification plan.
- Integration with the existing runtime-host-loading and runtime-service-core direction.

## Out of Scope

- Generic Linux microVM runner implementation.
- OCI lowering implementation.
- Treating Hermit as a native process or container.

## Verification

- `openspec validate implement-hermit-unikernel-profile --strict`
- Focused runtime-core or runner tests added by the implementation task.
- Docs/source-anchor tests where the change affects runtime architecture documentation.
- `git diff --check`
