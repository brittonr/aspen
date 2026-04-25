## ADDED Requirements

### Requirement: Alloc-safe cluster tickets default to transport-neutral bootstrap metadata

The unsigned `aspen-ticket` surface SHALL compile as an alloc-safe leaf crate by default. For this seam, the alloc-safe contract is the bare/default `default = []` feature surface and the explicit `--no-default-features` surface; those two surfaces SHALL remain equivalent. The unsigned cluster ticket payload SHALL store transport-neutral bootstrap metadata and a crate-local alloc-safe topic identifier instead of concrete iroh runtime types.

ID: architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata
#### Scenario: Bare cluster ticket dependency stays alloc-safe
ID: architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.bare-cluster-ticket-dependency-stays-alloc-safe

- **GIVEN** a contributor depends on `crates/aspen-ticket` with the bare/default feature set
- **WHEN** Cargo resolves and compiles the unsigned ticket core
- **THEN** the default feature set SHALL remain alloc-safe and equivalent to the explicit `--no-default-features` build surface
- **AND** both the bare/default surface and the explicit `--no-default-features` surface SHALL compile on `wasm32-unknown-unknown`
- **AND** the dependency graph SHALL exclude `iroh`, `iroh-gossip`, `rand`, and `anyhow`
- **AND** the unsigned ticket crate SHALL compile without runtime shell crates or ambient-clock-only APIs

#### Scenario: Cluster topic and bootstrap metadata roundtrip in alloc-safe form
ID: architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.topic-and-bootstrap-roundtrip-in-alloc-safe-form

- **GIVEN** an unsigned `AspenClusterTicket` built with the current alloc-safe topic type and `NodeAddress` bootstrap peers
- **WHEN** it is serialized and then deserialized
- **THEN** decoding SHALL succeed
- **AND** topic and bootstrap metadata SHALL be preserved across the roundtrip

#### Scenario: Default and explicit alloc-safe surfaces remain equivalent
ID: architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.default-and-explicit-alloc-safe-surfaces-remain-equivalent

- **GIVEN** the bare/default `default = []` feature surface and the explicit `--no-default-features` surface
- **WHEN** they are prepared for review
- **THEN** saved dependency and feature-tree artifacts SHALL show equivalent resolution for those two alloc-safe surfaces
- **AND** a dedicated equivalence artifact under `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/` SHALL record that comparison

#### Scenario: Alloc-safe topic identifier stays fixed-width and lossless at the iroh boundary
ID: architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.alloc-safe-topic-identifier-stays-fixed-width-and-lossless-at-the-iroh-boundary

- **GIVEN** the unsigned ticket stores a crate-local alloc-safe topic identifier
- **WHEN** ticket payloads are parsed or the topic crosses the optional `iroh` / gossip boundary
- **THEN** the stored topic identifier SHALL remain exactly 32 bytes with no hidden normalization
- **AND** malformed payloads that cannot decode that fixed-width topic SHALL fail through the crate-local ticket error surface
- **AND** conversion between the alloc-safe topic identifier and `iroh_gossip::proto::TopicId` SHALL be lossless when the payload is valid
- **AND** saved verification SHALL include one valid alloc-safe-topic ↔ `iroh_gossip::proto::TopicId` roundtrip proof and one malformed-topic rejection proof through the crate-local error type

#### Scenario: NodeAddress dependency edge stays alloc-safe
ID: architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-dependency-edge-stays-alloc-safe

- **GIVEN** the unsigned ticket imports `aspen_cluster_types::NodeAddress`
- **WHEN** that dependency edge is wired into `crates/aspen-ticket`
- **THEN** the `aspen-cluster-types` dependency SHALL use `default-features = false`
- **AND** the bare/default and explicit `--no-default-features` `aspen-ticket` surfaces SHALL not receive iroh runtime helpers through that edge unless `iroh` is explicitly selected

### Requirement: Cluster ticket runtime helpers require explicit shell opt-in

Runtime-only helpers for cluster tickets SHALL remain outside the alloc-safe default surface. Iroh conversion helpers and signed-ticket conveniences SHALL require explicit shell-side opt-in by runtime consumers.

ID: architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in
#### Scenario: Iroh conversion happens at the shell boundary
ID: architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.iroh-conversion-happens-at-the-shell-boundary

- **GIVEN** a runtime crate needs `iroh::EndpointAddr`, `iroh::EndpointId`, or `iroh_gossip::proto::TopicId` from an unsigned `AspenClusterTicket`
- **WHEN** it converts stored alloc-safe ticket metadata into runtime transport types
- **THEN** that crate SHALL opt into the explicit `iroh` feature in its own dependency stanza
- **AND** conversion SHALL happen at the runtime shell boundary instead of the alloc-safe default ticket surface

#### Scenario: Signed ticket support requires explicit opt-in
ID: architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.signed-ticket-support-requires-explicit-opt-in

- **GIVEN** a caller only needs unsigned cluster ticket parsing or transport-neutral storage
- **WHEN** it depends on `aspen-ticket`
- **THEN** signed-ticket types and explicit-time signed helpers SHALL remain unavailable until the explicit `signed` feature is enabled
- **AND** wall-clock / nonce-generating signed convenience wrappers SHALL remain unavailable until the `std` convenience bundle is enabled
- **AND** the `std` convenience bundle SHALL imply `signed` rather than creating an unrelated surface
- **AND** the `std` convenience bundle SHALL NOT enable `iroh` or `iroh-gossip` unless `iroh` is also explicitly selected
- **AND** saved verification SHALL prove the alloc-safe default side, the `--no-default-features --features signed` side, and the `--features std` side of that gate

#### Scenario: Signed-only surface stays distinct from std conveniences
ID: architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.signed-only-surface-stays-distinct-from-std-conveniences

- **GIVEN** a caller enables `--no-default-features --features signed`
- **WHEN** Cargo resolves and compiles that surface
- **THEN** signed explicit-time helpers SHALL remain available without `std`
- **AND** that surface SHALL avoid ambient clock reads, nonce generation helpers, and `rand` in its dependency graph until the `std` convenience bundle is also enabled
- **AND** that surface SHALL also exclude `iroh`, `iroh-gossip`, and `anyhow`
- **AND** review artifacts SHALL prove the signed-only surface separately from the `--features std` surface

#### Scenario: Cluster ticket seam proof is reviewable
ID: architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.cluster-ticket-seam-proof-is-reviewable

- **GIVEN** the cluster ticket boundary change is complete
- **WHEN** it is prepared for review
- **THEN** saved dependency, compile, and targeted test transcripts under `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/` SHALL include exact command/results for full-graph `cargo tree -p aspen-ticket -e normal`, full-graph `cargo tree -p aspen-ticket -e features`, full-graph `cargo tree -p aspen-ticket --no-default-features -e normal`, full-graph `cargo tree -p aspen-ticket --no-default-features -e features`, full-graph `cargo tree -p aspen-ticket --features iroh -e normal`, full-graph `cargo tree -p aspen-ticket --no-default-features --features signed -e normal`, full-graph `cargo tree -p aspen-ticket --features std -e normal`, `cargo check -p aspen-ticket`, `cargo check -p aspen-ticket --no-default-features`, `cargo check -p aspen-ticket --target wasm32-unknown-unknown`, `cargo check -p aspen-ticket --no-default-features --target wasm32-unknown-unknown`, `cargo check -p aspen-ticket --features iroh`, `cargo check -p aspen-ticket --no-default-features --features signed`, `cargo check -p aspen-ticket --no-default-features --features signed --target wasm32-unknown-unknown`, `cargo check -p aspen-ticket --features std`, and exact compile rails for every direct `aspen-ticket` consumer named in the audit scope
- **AND** those saved artifacts SHALL prove the bare/default and explicit `--no-default-features` unsigned surfaces exclude `iroh`, `iroh-gossip`, `rand`, and `anyhow`
- **AND** a dedicated artifact at `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/default-vs-no-default-equivalence.md` SHALL record the bare/default vs `--no-default-features` comparison
- **AND** a dedicated artifact at `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/workspace-dependency-proof.txt` SHALL prove the root `Cargo.toml` `aspen-ticket` workspace stanza explicitly sets `default-features = false`
- **AND** the saved direct-consumer audit SHALL preserve a repo-wide discovery proof derived from the workspace dependency graph (for example a `cargo metadata --format-version 1 --no-deps` query) so every direct `aspen-ticket` consumer is discovered regardless of manifest syntax, then classify each direct consumer, map each consumer to its exact compile evidence, and cite the shell-boundary conversion site for each consumer that opts into `iroh`
- **AND** saved artifacts SHALL also include representative transitive re-export leak proofs for downstream consumers reached through the `aspen-client` and `aspen-cluster` re-export paths so feature propagation stays reviewable beyond direct manifests
- **AND** `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/verification.md` SHALL map each checked task to those saved artifacts

### Requirement: Cluster ticket parse and validation errors stay alloc-safe and explicit

The unsigned `aspen-ticket` surface SHALL use crate-local error types for ticket parsing and validation. Malformed payloads and legacy unsigned payload layouts SHALL remain attributable through explicit ticket errors instead of `anyhow`-style runtime-only error wrapping.

ID: architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit
#### Scenario: Parse and validation failures use cluster ticket errors
ID: architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.parse-and-validation-failures-use-cluster-ticket-errors

- **GIVEN** a caller parses or validates a malformed unsigned cluster ticket in the shared crate
- **WHEN** decoding or field validation fails
- **THEN** the failure SHALL be reported through the crate-local cluster ticket error type
- **AND** the alloc-safe unsigned surface SHALL NOT require `anyhow`

#### Scenario: Legacy unsigned cluster tickets are rejected explicitly
ID: architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.legacy-unsigned-cluster-tickets-are-rejected-explicitly

- **GIVEN** an unsigned cluster ticket serialized with the pre-change runtime payload layout
- **WHEN** the alloc-safe `aspen-ticket` crate parses that payload
- **THEN** decoding SHALL fail through the crate-local unsigned ticket error surface
- **AND** the caller SHALL need to regenerate the cluster ticket using the current alloc-safe schema

#### Scenario: Unsigned wire-break proof is reviewable
ID: architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.unsigned-wire-break-proof-is-reviewable

- **GIVEN** the unsigned ticket payload changed from the pre-change runtime layout to the alloc-safe layout
- **WHEN** that wire break is prepared for review
- **THEN** a dedicated artifact at `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/unsigned-wire-break.md` SHALL show both current alloc-safe roundtrip success and explicit legacy unsigned payload rejection
- **AND** that proof SHALL be driven by a checked-in legacy unsigned fixture source at `crates/aspen-ticket/tests/legacy.rs`
- **AND** `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/verification.md` SHALL map those two proofs to the checked tasks
