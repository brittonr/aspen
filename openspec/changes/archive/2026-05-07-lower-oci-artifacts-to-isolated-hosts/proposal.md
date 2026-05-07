## Why

The archived runtime host-loading spec still treats `OciContainer` as a host kind. That risks baking a Podman/Docker-style plain container runner into Aspen as if it were a production isolation boundary. Aspen should instead treat OCI as an ingestion and compatibility artifact format, then lower verified OCI images into the strongest compatible Aspen host boundary: microVM by default, or Hyperlight/WASM/unikernel when the artifact can be transformed or rebuilt for that profile.

## What Changes

- **OCI role**: OCI images become content-addressed input artifacts, not first-class production host boundaries.
- **Default lowering**: production/risky/tenant OCI workloads must lower into `MicroVm` unless policy selects a stronger/smaller compatible target such as Hyperlight, WASM, or a unikernel profile.
- **Plain containers**: Podman/Docker-style host containers are excluded from the default production runtime; any future local/dev runner must be explicitly unsafe or dev-only and receipt-marked.
- **Receipts**: lowered runs must record the original OCI digest, selected lowering plan, derived rootfs/program/guest artifacts, host boundary, and redacted capability bindings.

## Capabilities

### Modified Capabilities

- `runtime-host-loading`: refines OCI from host boundary to artifact ingestion/lowering path.
- `runtime-capability-binding`: relies on the runtime-service-core capability-binding vocabulary; this change requires OCI lowering plans to preserve declared mounts, env handles, network policy, and capability-scoped host calls without adding a separate capability-binding delta.

## Impact

- **Specs**: Adds/modifies `runtime-host-loading` requirements for OCI artifact lowering and raw-container rejection.
- **Future APIs**: Guides replacement/demotion of `RuntimeHostKind::OciContainer` toward an `OciImage` artifact plus lowering plan and isolated host kind.
- **Security**: Prevents ordinary containers from being mistaken for Aspen's sandbox boundary.
- **Testing**: Future implementation should include admission tests rejecting production raw-container plans and accepting OCI-to-microVM lowering receipts.
