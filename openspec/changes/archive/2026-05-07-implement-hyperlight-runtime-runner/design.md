## Context

Aspen now has an accepted runtime host-loading baseline and an active runtime-service-core implementation OpenSpec. The Hyperlight runtime runner change owns one focused implementation seam so the runtime does not collapse distinct host boundaries into a vague generic runner.

## Goals

- Define Hyperlight runner capabilities and artifact verification.
- Specify host ABI/capability binding and fail-closed admission.
- Record execution outputs and lifecycle receipts.

## Non-Goals

- Replacing microVMs for all tenant workloads.
- Making every Aspen service Hyperlight-only.
- Implementing OCI lowering itself.

## Dependency Order

This change depends on `define-runtime-service-core` for shared service identity, lifecycle, route, health, capability-binding, and receipt vocabulary. It should drain before `lower-oci-artifacts-to-isolated-hosts`, which lowers OCI inputs into already-defined target host boundaries.

## Decisions

### 1. Keep this seam independent from OCI lowering

**Choice:** This change defines the target runner/profile contract directly; OCI lowering may target it later but is not implemented here.

**Rationale:** OCI is an input translation layer and should depend on proven isolated host targets rather than define them indirectly.

### 2. Fail closed before exposing runtime handles

**Choice:** Admission checks MUST complete before exposing route ownership, host calls, substrate handles, secrets, or mutable execution state.

**Rationale:** Runtime hosts are part of Aspen's security boundary and must not start from mutable tags, undeclared handles, or implicit ambient authority.

### 3. Receipts are operator evidence, not secret transport

**Choice:** Receipts record artifact identities, runner/profile identity, lifecycle state, resource summaries, and redacted capability handles.

**Rationale:** Runtime receipts may be persisted, replicated, exported, or shown to operators; they must never contain raw secrets.

## Risks / Trade-offs

- **Spec queue breadth**: Mitigate by keeping only spec-foundation tasks complete and implementation tasks focused.
- **Boundary confusion**: Mitigate by explicitly separating this runner/profile from OCI lowering and from the service-core control plane.
- **Overclaiming**: Mitigate by requiring evidence tasks before archive.

## Validation Plan

1. Validate this OpenSpec strictly.
2. Implement the smallest model/runner/profile slice.
3. Add focused positive and negative admission tests.
4. Update docs/source anchors if terminology changes.
5. Run targeted tests and whitespace checks before marking implementation tasks complete.
