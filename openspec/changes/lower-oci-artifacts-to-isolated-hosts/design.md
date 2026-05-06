## Context

Aspen's canonical runtime host-loading spec currently names `OciContainer` alongside real host boundaries. The intended architecture is sharper: OCI is a distribution/rootfs packaging format, while Aspen's runtime boundary should be NativeBuiltIn, NativeProcess, WASM, Hyperlight, MicroVm, or a VM-backed unikernel/guest profile. For dynamic remote workloads, the Fly.io/Kata-style path is OCI image/rootfs materialized into a VM boundary rather than direct Podman-style execution on the node host kernel.

## Goals / Non-Goals

**Goals:**

- Reclassify OCI as an artifact ingestion/lowering profile rather than a production host boundary.
- Make microVM-backed OCI the default for tenant, CI, risky adapter, and remote application workloads.
- Allow Hyperlight/WASM/unikernel lowering when the artifact is compatible or can be rebuilt/transformed with provenance.
- Require receipts to preserve both original OCI identity and lowered artifact/host identity.

**Non-Goals:**

- No implementation in this change.
- No commitment to a specific OCI unpacker, image registry client, or VM engine implementation.
- No Podman/Docker dependency in the production runtime contract.
- No claim that every OCI image can become Hyperlight, WASM, or a unikernel without rebuild constraints.

## Decisions

### 1. OCI is an input artifact, not a production host

**Choice:** `RuntimeArtifact::OciImage` remains useful, but production runtime admission SHALL lower it into an isolated host kind. The default remote/risky lowering target is `RuntimeHostKind::MicroVm`.

**Rationale:** OCI compatibility is valuable for existing Linux workloads, language runtimes, CI jobs, and migration paths. Ordinary containers share the host kernel and should not become Aspen's security boundary.

**Rejected alternative:** Baking Podman/Docker-style plain containers into the production runtime. That would pull Aspen toward Kubernetes-lite semantics and weaken the isolation/receipt story.

### 2. Lowering plans are explicit and receipt-backed

**Choice:** OCI admission SHALL produce a bounded lowering plan that records the selected target host, original digest, derived rootfs/program/guest artifacts, declared handles, and unsupported-image reason when lowering cannot proceed.

**Rationale:** Operators need to know what actually ran. The receipt must not only say `image: sha256:...`; it must say whether the image became a microVM rootfs, Hyperlight program, WASM component, or rejected plan.

### 3. Plain containers are dev/unsafe only if ever added

**Choice:** If Aspen later supports a raw local container runner, it SHALL be outside the default production host taxonomy and require explicit dev/unsafe policy plus receipt marking.

**Rationale:** This leaves room for fast local smokes without treating them as acceptable tenant/runtime isolation.

## Risks / Trade-offs

**Higher implementation cost than Podman** → Mitigate by first implementing OCI-to-microVM rootfs lowering, then optional Hyperlight/WASM/unikernel lowering only when specific artifact classes justify it.

**Some OCI images will not lower cleanly** → Mitigate with fail-closed admission, structured unsupported reasons, and clear requirements for entrypoint, filesystem, networking, init, and privilege expectations.

**MicroVM startup/resource overhead** → Mitigate with pooling/snapshotting later; do not weaken the production boundary for convenience.

## Validation Plan

- Strict OpenSpec validation for this change.
- Future runtime-core admission tests proving production `OciImage` without an isolated lowering target is rejected.
- Future serialization/redaction tests for lowering-plan receipts.
- Future docs/source-anchor tests keeping `docs/runtime-applications.md` aligned with OCI-as-artifact language.
