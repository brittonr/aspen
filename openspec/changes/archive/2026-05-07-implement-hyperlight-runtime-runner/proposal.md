## Why

Hyperlight is a preferred lighter isolated target for compatible native-ish workloads, but it needs its own runner/admission/receipt OpenSpec before services or OCI lowering can target it safely.

## What Changes

- Define Hyperlight runner capabilities and artifact verification.
- Specify host ABI/capability binding and fail-closed admission.
- Record execution outputs and lifecycle receipts.

## In Scope

- Active OpenSpec package for the Hyperlight runtime runner implementation seam.
- Requirements, design constraints, implementation tasks, and verification plan.
- Integration with the existing runtime-host-loading and runtime-service-core direction.

## Out of Scope

- Replacing microVMs for all tenant workloads.
- Making every Aspen service Hyperlight-only.
- Implementing OCI lowering itself.

## Verification

- `openspec validate implement-hyperlight-runtime-runner --strict`
- Focused runtime-core or runner tests added by the implementation task.
- Docs/source-anchor tests where the change affects runtime architecture documentation.
- `git diff --check`
