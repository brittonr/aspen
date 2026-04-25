## ADDED Requirements

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
- **AND** a dedicated equivalence artifact under `openspec/changes/archive/2026-04-25-alloc-safe-hooks-ticket/evidence/` SHALL record that comparison

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
- **THEN** saved dependency, compile, and targeted test transcripts under `openspec/changes/archive/2026-04-25-alloc-safe-hooks-ticket/evidence/` SHALL include full-graph `cargo tree -p aspen-hooks-ticket -e normal`, full-graph `cargo tree -p aspen-hooks-ticket -e features`, full-graph `cargo tree -p aspen-hooks-ticket --no-default-features -e normal`, full-graph `cargo tree -p aspen-hooks-ticket --no-default-features -e features`, `cargo check -p aspen-hooks-ticket`, `cargo check -p aspen-hooks-ticket --target wasm32-unknown-unknown`, `cargo check -p aspen-hooks-ticket --no-default-features`, `cargo check -p aspen-hooks-ticket --no-default-features --target wasm32-unknown-unknown`, `cargo check -p aspen-hooks-ticket --features std`, `cargo test -p aspen-hooks-ticket`, `cargo test -p aspen-hooks-ticket --test ui`, `cargo check -p aspen-hooks`, `cargo check -p aspen-cli`, `cargo tree -p aspen-hooks -e features -i aspen-cluster-types`, `cargo tree -p aspen-cli -e features -i aspen-cluster-types`, and targeted positive/negative regression tests for expiry plus runtime bootstrap-peer conversion in both `aspen-hooks` and `aspen-cli`
- **AND** those saved full-graph dependency artifacts SHALL prove both the bare/default graph and the explicit `--no-default-features` graph exclude `iroh`
- **AND** `openspec/changes/archive/2026-04-25-alloc-safe-hooks-ticket/verification.md` SHALL map each checked task to those saved artifacts

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
- **THEN** saved artifacts SHALL include targeted tests for invalid `default_payload` JSON, expired-ticket rejection, legacy-ticket rejection, and runtime legacy-decode surfacing under `openspec/changes/archive/2026-04-25-alloc-safe-hooks-ticket/evidence/`
- **AND** the saved full-graph dependency artifacts for `aspen-hooks-ticket` SHALL show the default and explicit alloc-safe surfaces exclude `anyhow`
- **AND** `openspec/changes/archive/2026-04-25-alloc-safe-hooks-ticket/verification.md` SHALL map those error-surface proofs to the checked tasks
