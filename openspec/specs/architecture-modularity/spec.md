## Purpose
Aspen maintains explicit crate boundaries, feature topology, and extraction contracts so reusable library surfaces stay independent from app/runtime shells unless explicitly opted in.
## Requirements
### Requirement: Acyclic no-std core boundary

Aspen MUST maintain an acyclic dependency boundary where the alloc-only `aspen-core` surface depends only on alloc-safe leaf crates, and `std`-bound runtime shells depend on that core rather than the reverse.

ID: architecture.modularity.acyclic-no-std-core-boundary
#### Scenario: Cluster types default to alloc-safe builds
ID: architecture.modularity.acyclic-no-std-core-boundary.cluster-types-default-to-alloc-safe-builds

- **GIVEN** `aspen-cluster-types` on the alloc-only foundation path
- **WHEN** the crate is built with its default production configuration or `--no-default-features`
- **THEN** it MUST compile as an alloc-safe crate without `iroh-base` in its bare/default production graph
- **AND** runtime conversion helpers MUST remain unavailable unless the consumer opts into the `iroh` feature explicitly

#### Scenario: Cluster-type verification is reviewable
ID: architecture.modularity.acyclic-no-std-core-boundary.cluster-types-verification-is-reviewable

- **GIVEN** the alloc-safe-by-default contract for `aspen-cluster-types`
- **WHEN** the seam is verified for review
- **THEN** the exact commands and results for `cargo check -p aspen-cluster-types --no-default-features`, `cargo check -p aspen-cluster-types --no-default-features --target wasm32-unknown-unknown`, `cargo test -p aspen-cluster-types`, `cargo test -p aspen-cluster-types --features iroh`, `cargo tree -p aspen-cluster-types -e normal --depth 2`, `cargo tree -p aspen-cluster-types --features iroh -e normal --depth 2`, and `cargo tree -p aspen-traits -e normal --depth 2` SHALL be saved under `openspec/changes/cluster-types-iroh-opt-in/evidence/cluster-types-validation.md`
- **AND** the saved bare/default tree artifact SHALL prove `iroh-base` is absent until `--features iroh` is enabled
- **AND** the saved alloc-safe consumer-path artifact for `aspen-traits` SHALL prove a bare/default workspace consumer path does not receive `iroh` implicitly
- **AND** the exact root-workspace stanza proof for `Cargo.toml` SHALL be saved under `openspec/changes/cluster-types-iroh-opt-in/evidence/workspace-dependency-proof.txt`
- **AND** `openspec/changes/cluster-types-iroh-opt-in/verification.md` SHALL identify which artifact proves each cluster-types claim

### Requirement: Feature bundles are explicit and bounded

The build feature graph SHALL distinguish alloc-only core surfaces, `std`-only shell features, and higher-level convenience bundles so enabling foundational Aspen contracts does not silently pull runtime dependencies.

ID: architecture.modularity.feature-bundles-are-explicit-and-bounded
#### Scenario: Cluster-type runtime helpers require per-crate opt-in
ID: architecture.modularity.feature-bundles-are-explicit-and-bounded.cluster-types-iroh-opt-in-is-explicit

- **GIVEN** a crate that directly depends on `aspen-cluster-types`
- **WHEN** it needs runtime helper APIs such as `NodeAddress::new`, `ClusterNode::with_iroh_addr`, `.iroh_addr()`, or `try_into_iroh()`
- **THEN** that crate MUST opt into `features = ["iroh"]` in its own dependency stanza
- **AND** workspace-level dependency defaults MUST NOT re-enable `iroh` implicitly for unrelated consumers
- **AND** a deterministic direct-consumer audit generated from `rg 'aspen-cluster-types\\s*=\\s*\\{' . -g 'Cargo.toml'` SHALL be saved under `openspec/changes/cluster-types-iroh-opt-in/evidence/direct-consumer-audit.md`
- **AND** that audit SHALL classify every direct consumer as either `iroh-opt-in` or `alloc-safe`, cite helper API use or non-use from `rg 'NodeAddress::new|ClusterNode::with_iroh_addr|\\.iroh_addr\\(|try_into_iroh\\(' . -g '*.rs'`, and map every consumer to saved evidence or an explicit deterministic validation rule
- **AND** every `iroh-opt-in` classification in the audit SHALL correspond to an explicit consumer manifest stanza after workspace-level feature removal rather than inherited workspace defaults
- **AND** the saved runtime/test consumer artifact SHALL include exact command/results for every direct consumer whose manifest classification changed in this seam

### Requirement: Extraction inventory tracks coordination family
The crate extraction inventory at `docs/crate-extraction.md` SHALL include the coordination family with current readiness state, manifest link, owner status, and next action for both `aspen-coordination` and `aspen-coordination-protocol`.

ID: architecture-modularity.extraction-inventory-tracks-coordination

#### Scenario: Inventory row exists for coordination
- **WHEN** `docs/crate-extraction.md` is read
- **THEN** the broader candidate inventory table SHALL include a row for the coordination family
- **AND** the row SHALL link to `docs/crate-extraction/coordination.md`
- **AND** the readiness state SHALL reflect the verified extraction-readiness status

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
- **AND** a dedicated equivalence artifact under `openspec/changes/alloc-safe-cluster-ticket/evidence/` SHALL record that comparison

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
- **THEN** saved dependency, compile, and targeted test transcripts under `openspec/changes/alloc-safe-cluster-ticket/evidence/` SHALL include exact command/results for full-graph `cargo tree -p aspen-ticket -e normal`, full-graph `cargo tree -p aspen-ticket -e features`, full-graph `cargo tree -p aspen-ticket --no-default-features -e normal`, full-graph `cargo tree -p aspen-ticket --no-default-features -e features`, full-graph `cargo tree -p aspen-ticket --features iroh -e normal`, full-graph `cargo tree -p aspen-ticket --no-default-features --features signed -e normal`, full-graph `cargo tree -p aspen-ticket --features std -e normal`, `cargo check -p aspen-ticket`, `cargo check -p aspen-ticket --no-default-features`, `cargo check -p aspen-ticket --target wasm32-unknown-unknown`, `cargo check -p aspen-ticket --no-default-features --target wasm32-unknown-unknown`, `cargo check -p aspen-ticket --features iroh`, `cargo check -p aspen-ticket --no-default-features --features signed`, `cargo check -p aspen-ticket --no-default-features --features signed --target wasm32-unknown-unknown`, `cargo check -p aspen-ticket --features std`, and exact compile rails for every direct `aspen-ticket` consumer named in the audit scope
- **AND** those saved artifacts SHALL prove the bare/default and explicit `--no-default-features` unsigned surfaces exclude `iroh`, `iroh-gossip`, `rand`, and `anyhow`
- **AND** a dedicated artifact at `openspec/changes/alloc-safe-cluster-ticket/evidence/default-vs-no-default-equivalence.md` SHALL record the bare/default vs `--no-default-features` comparison
- **AND** a dedicated artifact at `openspec/changes/alloc-safe-cluster-ticket/evidence/workspace-dependency-proof.txt` SHALL prove the root `Cargo.toml` `aspen-ticket` workspace stanza explicitly sets `default-features = false`
- **AND** the saved direct-consumer audit SHALL preserve a repo-wide discovery proof derived from the workspace dependency graph (for example a `cargo metadata --format-version 1 --no-deps` query) so every direct `aspen-ticket` consumer is discovered regardless of manifest syntax, then classify each direct consumer, map each consumer to its exact compile evidence, and cite the shell-boundary conversion site for each consumer that opts into `iroh`
- **AND** saved artifacts SHALL also include representative transitive re-export leak proofs for downstream consumers reached through the `aspen-client` and `aspen-cluster` re-export paths so feature propagation stays reviewable beyond direct manifests
- **AND** `openspec/changes/alloc-safe-cluster-ticket/verification.md` SHALL map each checked task to those saved artifacts

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
- **THEN** a dedicated artifact at `openspec/changes/alloc-safe-cluster-ticket/evidence/unsigned-wire-break.md` SHALL show both current alloc-safe roundtrip success and explicit legacy unsigned payload rejection
- **AND** that proof SHALL be driven by a checked-in legacy unsigned fixture source at `crates/aspen-ticket/tests/legacy.rs`
- **AND** `openspec/changes/alloc-safe-cluster-ticket/verification.md` SHALL map those two proofs to the checked tasks

### Requirement: Alloc-safe hook tickets default to transport-neutral bootstrap metadata

The `aspen-hooks-ticket` crate SHALL compile as an alloc-safe leaf crate by default. For this seam, the alloc-safe contract is the bare/default `default = []` feature surface and the explicit `--no-default-features` build surface; those two surfaces SHALL remain equivalent. Its shared `AspenHookTicket` payload SHALL store bootstrap peers as transport-neutral `NodeAddress` values and SHALL expose deterministic expiry helpers that accept explicit time inputs instead of requiring ambient wall-clock reads.

ID: architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata
#### Scenario: Bare hook ticket dependency stays alloc-safe
ID: architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.bare-hook-ticket-dependency-stays-alloc-safe

- **GIVEN** a contributor depends on `crates/aspen-hooks-ticket` with the bare/default feature set
- **WHEN** Cargo resolves and compiles that crate
- **THEN** the default feature set SHALL remain alloc-safe and equivalent to the explicit alloc-safe build contract
- **AND** the dependency graph SHALL exclude `iroh`
- **AND** the ticket crate SHALL compile without runtime shell crates or ambient-clock-only APIs

#### Scenario: Expiry math stays testable without wall clock
ID: architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.expiry-math-stays-testable-without-wall-clock

- **GIVEN** a caller needs to compute or validate ticket expiry in an alloc-safe environment
- **WHEN** it uses the hook ticket's expiry APIs
- **THEN** the crate SHALL provide deterministic helpers that accept `now_secs` explicitly
- **AND** expiration checks SHALL remain available without reading the system clock

#### Scenario: NodeAddress hook tickets roundtrip successfully
ID: architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-hook-tickets-roundtrip-successfully

- **GIVEN** an `AspenHookTicket` built with `NodeAddress` bootstrap peers under the current schema
- **WHEN** it is serialized and then deserialized
- **THEN** decoding SHALL succeed
- **AND** bootstrap peer metadata SHALL be preserved across the roundtrip

#### Scenario: Default and explicit alloc-safe surfaces remain equivalent
ID: architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.default-and-explicit-alloc-safe-surfaces-remain-equivalent

- **GIVEN** the bare/default `default = []` feature surface and the explicit `--no-default-features` surface
- **WHEN** they are prepared for review
- **THEN** saved dependency and feature-tree artifacts SHALL show equivalent resolution for those two alloc-safe surfaces
- **AND** a dedicated equivalence artifact under `openspec/changes/alloc-safe-hooks-ticket/evidence/` SHALL record that comparison

#### Scenario: NodeAddress dependency edge stays alloc-safe
ID: architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-dependency-edge-stays-alloc-safe

- **GIVEN** `aspen-hooks-ticket` imports `NodeAddress` from `aspen-cluster-types`
- **WHEN** that manifest dependency edge is wired
- **THEN** the `aspen-cluster-types` dependency in `crates/aspen-hooks-ticket/Cargo.toml` SHALL disable default features
- **AND** that edge SHALL NOT opt into iroh conversion helpers

### Requirement: Hook ticket runtime helpers require explicit shell opt-in

Runtime-only helpers for `AspenHookTicket` SHALL remain outside the alloc-safe default surface. Wall-clock convenience wrappers and iroh-native connection setup SHALL require explicit shell-side opt-in by runtime consumers.

ID: architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in
#### Scenario: Runtime conversion happens at the shell boundary
ID: architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.runtime-conversion-happens-at-the-shell-boundary

- **GIVEN** runtime crates `aspen-hooks` and `aspen-cli` need to connect using an `AspenHookTicket`
- **WHEN** they turn stored bootstrap peers into iroh connection targets
- **THEN** each crate SHALL convert `NodeAddress` values at its runtime shell boundary through a direct runtime dependency that opts into `aspen-cluster-types` iroh conversion helpers
- **AND** invalid bootstrap peers SHALL produce explicit runtime errors instead of silently assuming a valid iroh endpoint

#### Scenario: Std convenience wrappers require explicit opt-in
ID: architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.std-convenience-wrappers-require-explicit-opt-in

- **GIVEN** the bare/default `aspen-hooks-ticket` build and a caller that only needs alloc-safe ticket logic
- **WHEN** wall-clock convenience helpers are compiled or used
- **THEN** those helpers SHALL remain unavailable until the explicit `std` feature is enabled
- **AND** saved verification SHALL prove both the default alloc-safe side and the `--features std` side of that gate

#### Scenario: Hook ticket seam proof is reviewable
ID: architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.hook-ticket-seam-proof-is-reviewable

- **GIVEN** the hook ticket boundary change is complete
- **WHEN** it is prepared for review
- **THEN** saved dependency, compile, and targeted test transcripts under `openspec/changes/alloc-safe-hooks-ticket/evidence/` SHALL include full-graph `cargo tree -p aspen-hooks-ticket -e normal`, full-graph `cargo tree -p aspen-hooks-ticket -e features`, full-graph `cargo tree -p aspen-hooks-ticket --no-default-features -e normal`, full-graph `cargo tree -p aspen-hooks-ticket --no-default-features -e features`, `cargo check -p aspen-hooks-ticket`, `cargo check -p aspen-hooks-ticket --target wasm32-unknown-unknown`, `cargo check -p aspen-hooks-ticket --no-default-features`, `cargo check -p aspen-hooks-ticket --no-default-features --target wasm32-unknown-unknown`, `cargo check -p aspen-hooks-ticket --features std`, `cargo test -p aspen-hooks-ticket`, `cargo test -p aspen-hooks-ticket --test ui`, `cargo check -p aspen-hooks`, `cargo check -p aspen-cli`, `cargo tree -p aspen-hooks -e features -i aspen-cluster-types`, `cargo tree -p aspen-cli -e features -i aspen-cluster-types`, and targeted positive/negative regression tests for expiry plus runtime bootstrap-peer conversion in both `aspen-hooks` and `aspen-cli`
- **AND** those saved full-graph dependency artifacts SHALL prove both the bare/default graph and the explicit `--no-default-features` graph exclude `iroh`
- **AND** `openspec/changes/alloc-safe-hooks-ticket/verification.md` SHALL map each checked task to those saved artifacts

### Requirement: Hook ticket parse and validation errors stay alloc-safe and explicit

The shared `aspen-hooks-ticket` crate SHALL use a crate-local error surface for ticket parsing and validation. Malformed payloads, invalid `default_payload` JSON, and expired-ticket checks SHALL remain attributable through explicit ticket errors instead of `anyhow`-style runtime-only error wrapping.

ID: architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit
#### Scenario: Parse and validation failures use hook ticket errors
ID: architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.parse-and-validation-failures-use-hook-ticket-errors

- **GIVEN** a caller parses or validates a malformed hook ticket in the shared crate
- **WHEN** decoding, `default_payload` JSON validation, or expiry validation fails
- **THEN** the failure SHALL be reported through the crate-local hook ticket error type
- **AND** the alloc-safe crate SHALL NOT require `anyhow` to surface those bounded parse and validation errors

#### Scenario: Legacy serialized hook tickets are rejected explicitly
ID: architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.legacy-serialized-hook-tickets-are-rejected-explicitly

- **GIVEN** a hook ticket serialized with the pre-change `EndpointAddr` payload schema
- **WHEN** the alloc-safe `aspen-hooks-ticket` crate parses that payload
- **THEN** decoding SHALL fail through the crate-local hook ticket error surface
- **AND** the caller SHALL need to regenerate the hook ticket using the current `NodeAddress` schema

#### Scenario: Runtime consumers surface legacy decode failures explicitly
ID: architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.runtime-consumers-surface-legacy-decode-failures-explicitly

- **GIVEN** `aspen-hooks` or `aspen-cli` receives a legacy serialized hook ticket string
- **WHEN** shared ticket decoding fails before any runtime `NodeAddress` conversion
- **THEN** the consumer SHALL return an explicit parse/decode error to the caller
- **AND** it SHALL NOT silently reinterpret or suppress the failure

#### Scenario: Hook ticket error-surface proof is reviewable
ID: architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.hook-ticket-error-surface-proof-is-reviewable

- **GIVEN** the parse, validation, expiry, and legacy-ticket rejection contract
- **WHEN** the seam is prepared for review
- **THEN** saved artifacts SHALL include targeted tests for invalid `default_payload` JSON, expired-ticket rejection, legacy-ticket rejection, and runtime legacy-decode surfacing under `openspec/changes/alloc-safe-hooks-ticket/evidence/`
- **AND** the saved full-graph dependency artifacts for `aspen-hooks-ticket` SHALL show the default and explicit alloc-safe surfaces exclude `anyhow`
- **AND** `openspec/changes/alloc-safe-hooks-ticket/verification.md` SHALL map those error-surface proofs to the checked tasks

