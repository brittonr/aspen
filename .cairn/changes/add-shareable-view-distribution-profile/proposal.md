## Why

Molten already provides content-addressed storage, receiver-driven DAG synchronization, partial verified progress, retention controls, capability checks, and distributed object references. It does not have a profile for distributing schema-first Shareable Views.

Tile's current surface transport sends terminal grids or pixel buffers. The proposed Tile contract instead references typed state, dependencies, actions, presentation profiles, privacy facts, and optional live behavior.

Molten needs a workload-neutral distribution profile for this contract. The profile must transfer and retain only authorized content. It must not own UI schemas, rendering, semantic actions, fork meaning, or application state.

## What Changes

- Add a versioned Shareable View distribution request over one exact published application manifest profile.
- Reuse existing content-store and DAG-sync cores for complete dependency-closure transfer, verification, partial progress, repair, and cancellation.
- Require read and privacy admission before local availability lookup, reuse, range planning, or payload reveal.
- Partition retained objects by privacy domain and issuer policy without global presence queries or cross-domain deduplication.
- Add a snapshot-first live channel for revision-fenced state deltas and opaque semantic-action request and outcome envelopes.
- Add an optional fork-request route that forwards one authorized request to the current application owner without defining clone semantics.
- Measure repeated dependency transfer before any cache or reuse support receives a product claim.
- Add deterministic simulation and live Iroh fixtures for closure reuse, gaps, corruption, revocation, uncertainty, and cleanup.

## Capabilities

### New Capabilities

- `molten.shareable_view_distribution.boundary`: Molten distributes Shareable View artifacts without owning their semantic meaning.
- `molten.shareable_view_distribution.closure`: Receiver-driven transfer resolves one verified dependency closure under explicit limits.
- `molten.shareable_view_distribution.privacy`: Authority and privacy checks occur before availability or reuse observations.
- `molten.shareable_view_distribution.reuse`: Receiver-local reuse remains domain-partitioned, measured, and non-authoritative.
- `molten.shareable_view_distribution.live`: Snapshot-first live state and action envelopes preserve order and uncertainty.
- `molten.shareable_view_distribution.fork`: Fork requests route to the current owner without granting clone authority.
- `molten.shareable_view_distribution.validation`: Simulation and live evidence cover positive and negative behavior.

## Success Criteria

- A receiver obtains one complete authorized Shareable View closure and verifies every member before exposure.
- A compatible reconnect resumes from exact verified progress without retransferring admitted complete members.
- A local object match remains invisible until current read and privacy admission passes.
- Cross-domain storage never exposes a reusable presence oracle or shared hit result.
- A live route accepts one complete baseline before ordered deltas or semantic actions.
- A stale or uncertain mutating action is not replayed automatically.
- A fork request reaches only the current admitted owner and grants no clone semantics.

## False Completion

A transport connection, Iroh ticket, blob hash, cache hit, complete download, or successful action delivery is not application authority.

A benchmark with only synthetic duplicate assets is not enough to enable product reuse by default.

A world snapshot, process image, pixel stream, or shared backend handle is not a Shareable View dependency closure.

## Impact

- **Core**: Distribution request, closure roles, privacy-domain admission, live stream, action, fork, and receipt decisions.
- **Shell**: Existing content, DAG-sync, Iroh, storage, authority, retention, and observability ports.
- **Policy**: Typed Nickel bounds for manifests, members, bytes, privacy domains, reuse, live streams, actions, and receipts.
- **Simulation**: Corruption, gaps, revocation, cancellation, disconnect, restart, and cleanup cases.
- **Evidence**: Transfer counts, verified bytes, reused members, privacy outcomes, uncertainty, and non-claims.

## Dependencies

Implementation requires an exact published Tile Shareable View contract. A Kamacite envelope can be admitted only through a separate exact published adapter profile.

Molten will reuse its existing content-store, DAG-sync, fabric transport, authority, retention, and simulation contracts. It must not copy their logic into a UI-specific subsystem.

## Non-goals

- Owning Tile manifests, application schemas, presentation profiles, renderer behavior, accessibility meaning, semantic action meaning, or fork semantics.
- Global cache inventories, peer cache advertisements, cross-domain deduplication, or digest-based authorization.
- Automatic execution of renderer code, Wasm components, callbacks, or application commands.
- Transparent process, heap, PTY, Wayland, GPU, socket, credential, or host-handle transfer.
- Exactly-once action effects, global convergence, confidentiality, arbitrary application support, or production readiness.
