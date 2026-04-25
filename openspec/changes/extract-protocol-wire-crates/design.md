## Context

The protocol/wire family contains request/response schemas and domain wire types used by clients, handlers, and tools. `ClientRpcRequest` / `ClientRpcResponse` postcard discriminants are append-only compatibility contracts. Prior boundary work moved auth types toward `aspen-auth-core` and made several protocol crates alloc-friendlier, but the extraction inventory still marks the family as `workspace-internal` and the family lacks durable golden compatibility rails.

`aspen-coordination-protocol` has already been proven standalone as part of coordination extraction. This change treats it as a member of the family but avoids reworking it unless shared policy/checker changes require small updates.

## Goals / Non-Goals

**Goals:**

- Prove protocol crates compile independently of handler registries and runtime shells.
- Preserve wire compatibility with golden postcard evidence.
- Keep portable type dependencies pointed at leaf crates, not shell crates.
- Document feature contracts and forbidden dependency boundaries.
- Provide a downstream serialization fixture using canonical protocol crate imports.

**Non-Goals:**

- Changing protocol semantics or adding new RPC operations.
- Reordering existing postcard enum variants.
- Replacing postcard/serde.
- Extracting handler implementations or transports.
- Publishing crates or splitting repositories.

## Decisions

### Decision 1: Treat append-only discriminants as first-class compatibility contract

**Choice:** Add golden/snapshot tests or checked fixtures for public wire enums and schema-critical domain protocol types.

**Rationale:** A protocol crate can compile cleanly and still break clients by shifting postcard discriminants. Compatibility must be tested explicitly.

**Alternative:** Rely on source review for enum changes. Rejected because this repo already has a hard compatibility note for `ClientRpcRequest` / `ClientRpcResponse` and needs deterministic rails.

### Decision 2: Portable dependencies only by default

**Choice:** Default protocol features may depend on leaf type crates and other protocol crates, but not handler registries, runtime auth shells, node bootstrap, concrete transport, SQL engines, trust/secrets services, UI/web binaries, or root Aspen.

**Rationale:** Wire schemas should be usable by clients and test fixtures that do not run an Aspen node.

**Alternative:** Allow runtime crates behind default features and depend on consumers to disable them. Rejected because feature unification makes that fragile.

### Decision 3: Use family-level manifest with per-crate sections

**Choice:** Keep `docs/crate-extraction/protocol-wire.md` as one family manifest with per-crate tables for feature sets, dependencies, compatibility tests, and readiness state.

**Rationale:** The crates share one purpose and one compatibility policy, but each crate has different optional domains.

**Alternative:** One manifest per protocol crate immediately. Rejected as unnecessary ceremony until one crate needs a separate publication path.

## Risks / Trade-offs

- **[Risk] Snapshot churn hides real compatibility breaks** → Mitigate by requiring reviewable golden baselines and append-only tests rather than blind snapshot acceptance.
- **[Risk] Optional domain features pull runtime crates** → Mitigate with dependency-boundary checks per feature set.
- **[Risk] wasm checks fail because tests use std serializers** → Mitigate by separating production no-std/alloc checks from test-only `postcard::to_stdvec` paths or switching tests to alloc serializers.
- **[Trade-off] Coordination protocol appears in two manifests** → Acceptable; the protocol family manifest links to coordination evidence instead of duplicating proof.
