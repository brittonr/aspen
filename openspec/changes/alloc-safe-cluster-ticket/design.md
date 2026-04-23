## Context

`aspen-ticket` currently combines three different concerns in one always-on surface:

1. unsigned cluster bootstrap metadata,
2. runtime iroh / iroh-gossip conversion helpers, and
3. signed-ticket convenience code with ambient time and random nonce generation.

The unsigned payload still stores concrete iroh endpoint metadata (`EndpointId` / `EndpointAddr`) and a concrete gossip `TopicId`, so even callers that only need a shareable ticket string inherit runtime transport crates. The crate also still uses `anyhow` for parse/validation and `unwrap_or_default()` in the signed encoder's `to_bytes()` path.

Direct `aspen-ticket` consumers today are:

- `crates/aspen-ci-executor-vm`
- `crates/aspen-cluster-handler`
- `crates/aspen-cluster`
- `crates/aspen-rpc-handlers`
- `crates/aspen-client`
- `crates/aspen-ci`
- root workspace dependency stanza in `Cargo.toml`

## Goals / Non-Goals

**Goals:**
- Make the unsigned `aspen-ticket` surface alloc-safe by default.
- Replace public unsigned ticket transport metadata with alloc-safe `NodeAddress` plus a crate-local alloc-safe topic identifier.
- Keep runtime iroh / gossip conversions behind explicit feature opt-in.
- Keep signed-ticket support explicit instead of unconditional.
- Replace `anyhow` parse/validation results with crate-local ticket errors.
- Save deterministic review artifacts for both the crate and every changed direct consumer.

**Non-Goals:**
- Reworking Aspen cluster discovery semantics.
- Changing hook ticket behavior again.
- Providing mixed-schema backward compatibility for old unsigned ticket payloads.
- Removing signed tickets entirely.

## Decisions

### 1. Unsigned ticket core stores alloc-safe topic and bootstrap metadata

**Choice:** the unsigned `AspenClusterTicket` payload will store a crate-local `ClusterTopicId([u8; 32])` and transport-neutral bootstrap peers as `Vec<aspen_cluster_types::NodeAddress>`.

**Boundary contract:** `ClusterTopicId` is exactly 32 bytes, preserves bytes as-is with no normalization or canonicalization step, and converts to/from `iroh_gossip::proto::TopicId` by direct byte copy only. Malformed payloads that cannot decode that fixed-width topic will fail through the crate-local unsigned ticket error surface instead of falling back or normalizing.

**Rationale:** those two fields are the public runtime leaks in the unsigned ticket surface. `NodeAddress` already gives Aspen an alloc-safe endpoint contract, and a fixed-size topic newtype removes the default `iroh-gossip::proto::TopicId` dependency from the shared payload.

**Alternative considered:** keep the existing fields and only add wrapper helpers. Rejected because the default crate graph would still resolve runtime transport crates.

### 2. Runtime conversion helpers move behind an explicit `iroh` feature

**Choice:** constructors/accessors that use `iroh::EndpointAddr`, `iroh::EndpointId`, `iroh::TransportAddr`, or `iroh_gossip::proto::TopicId` will live behind an explicit `iroh` feature. The alloc-safe default surface will use only `ClusterTopicId` and `NodeAddress`.

**Rationale:** runtime crates still need compatibility helpers, but the shared ticket crate should not pull them by default.

**Implementation direction:** direct consumers that actually use those helpers will opt into `aspen-ticket = { default-features = false, features = ["iroh"] }` locally instead of inheriting runtime defaults from the workspace stanza.

### 3. Signed-ticket support uses an explicit `signed` + `std` contract

**Choice:** the crate feature map will be `default = []`, `iroh`, `signed`, and `std`, with `std` explicitly implying `signed`. `SignedAspenClusterTicket` and explicit-time signed helpers live behind `signed`. Wall-clock / nonce-generating convenience wrappers live behind `std`. The supported review surfaces are therefore: bare/default, `--no-default-features`, `--features iroh`, `--no-default-features --features signed`, and `--features std`.

**Rationale:** this keeps the unsigned default surface alloc-safe, keeps signed-ticket logic explicit, and still gives runtime shells an ergonomic opt-in path.

**Alternative considered:** let `signed` implicitly mean `std`. Rejected because the proposal and FCIS goals need an explicit line between pure signed helpers and ambient-time/runtime conveniences.

### 4. Signed ticket time logic follows FCIS within the feature-gated shell

**Choice:** signed-ticket helpers behind `signed` will expose explicit-time entry points (`verify_at`, `is_expired_at`, and supporting pure timestamp checks), while `std` adds only convenience wrappers that read the current clock and generate nonce material (`sign`, `sign_with_validity`, `verify`, `is_expired`).

**Rationale:** even inside a feature-gated runtime module, the time math should stay testable without wall-clock reads.

**Alternative considered:** leave ambient `SystemTime` reads inside the only verification path. Rejected because the pure timestamp checks are the actual unit under test.

### 5. Use crate-local ticket errors for the alloc-safe surface

**Choice:** replace unsigned parse/validation `anyhow::Result` APIs with a dedicated `ClusterTicketError` enum. Signed-ticket helpers may use a sibling feature-gated error type or shared variants, but the alloc-safe unsigned surface will stay explicit and attributable.

**Rationale:** alloc-safe shared crates should not require `anyhow` to report bounded parse/validation failures.

### 6. Signed encoder fails loudly instead of returning empty bytes

**Choice:** `impl Ticket for SignedAspenClusterTicket::to_bytes()` will use an `expect(...)`-style invariant break with contextual text such as `"SignedAspenClusterTicket serialization is infallible for bounded fields"` instead of `unwrap_or_default()`.

**Rationale:** empty payload fallback hides impossible serializer bugs and can mint misleading signed tickets. A deterministic `expect(...)` path is auditable in source and unambiguous in failure behavior.

### 7. Root workspace and every changed direct consumer must prove explicit feature topology

**Choice:** remove implicit runtime enablement from the root `Cargo.toml` workspace dependency stanza, classify every direct consumer of `aspen-ticket`, and save exact compile rails for every direct consumer whose manifest classification changes in this seam.

**Direct-consumer feature matrix (derived from `rg -n 'aspen-ticket\\s*=\\s*\\{' . -g 'Cargo.toml'` plus the helper-usage audit command below):**

| Consumer | Current use shape | Planned `aspen-ticket` features | Manifest changes? | Compile rail |
| --- | --- | --- | --- | --- |
| `Cargo.toml` workspace stanza | shared source for workspace consumers | explicit alloc-safe workspace stanza (`default-features = false`) | yes | `rg -n '^aspen-ticket\s*=\s*\{' Cargo.toml` |
| `crates/aspen-ci-executor-vm` | deserialize unsigned ticket for VM lifecycle/bootstrap flows | `iroh` | yes | `cargo check -p aspen-ci-executor-vm` |
| `crates/aspen-cluster-handler` | construct unsigned tickets from runtime endpoint metadata | `iroh` | yes | `cargo check -p aspen-cluster-handler --tests` |
| `crates/aspen-cluster` | re-export unsigned parse helpers and signed ticket surface for runtime cluster paths | `iroh`, `std` | yes | `cargo check -p aspen-cluster` |
| `crates/aspen-rpc-handlers` | direct dependency only; no ticket helper hits in the helper-usage audit | bare/default (workspace-inherited) | no local stanza change expected | `cargo check -p aspen-rpc-handlers` |
| `crates/aspen-client` | deserialize unsigned tickets and connect through bootstrap peers | `iroh` | yes | `cargo check -p aspen-client` |
| `crates/aspen-ci` | direct dependency only; no ticket helper hits in the helper-usage audit | bare/default (workspace-inherited) | no local stanza change expected | `cargo check -p aspen-ci` |

**Deterministic audit commands and artifact paths:**
- Save `rg -n 'aspen-ticket\s*=\s*\{' . -g 'Cargo.toml'` under `openspec/changes/alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md`.
- Save `rg -n 'SignedAspenClusterTicket|parse_ticket_to_addrs|with_bootstrap_addr|with_bootstrap\(|endpoint_addrs\(|endpoint_ids\(|AspenClusterTicket::deserialize|AspenClusterTicket::new|iroh::EndpointAddr|iroh::EndpointId|iroh_gossip::proto::TopicId|ClusterTopicId|try_into_iroh|to_topic_id|from_topic_id' crates/aspen-ci-executor-vm crates/aspen-cluster-handler crates/aspen-cluster crates/aspen-rpc-handlers crates/aspen-client crates/aspen-ci -g '*.rs'` alongside the same audit artifact to justify the feature classification.
- In that same audit artifact, record one exact source citation for each `iroh` consumer (`crates/aspen-ci-executor-vm`, `crates/aspen-cluster-handler`, `crates/aspen-cluster`, `crates/aspen-client`) showing the shell-boundary conversion site, then map each citation to its compile rail.
- Save a deterministic negative assertion for the root workspace stanza under `openspec/changes/alloc-safe-cluster-ticket/evidence/workspace-dependency-proof.txt` that fails review if the `Cargo.toml` `aspen-ticket` workspace stanza reintroduces `iroh`, `signed`, or `std` defaults.
- Treat `crates/aspen-rpc-handlers` and `crates/aspen-ci` as bare/default only if that saved audit artifact remains empty for their source trees; otherwise reopen this decision before implementation tasks are checked.

**Rationale:** the earlier `aspen-cluster-types` seam showed that workspace inheritance can silently re-enable runtime helpers unless every direct consumer is audited explicitly.

### 8. Save exact verification rails

**Choice:** save exact review artifacts under `openspec/changes/alloc-safe-cluster-ticket/evidence/` for:

- full-graph `cargo tree -p aspen-ticket -e normal`
- full-graph `cargo tree -p aspen-ticket -e features`
- full-graph `cargo tree -p aspen-ticket --no-default-features -e normal`
- full-graph `cargo tree -p aspen-ticket --no-default-features -e features`
- full-graph `cargo tree -p aspen-ticket --features iroh -e normal`
- full-graph `cargo tree -p aspen-ticket --no-default-features --features signed -e normal`
- full-graph `cargo tree -p aspen-ticket --features std -e normal`
- `cargo check -p aspen-ticket`
- `cargo check -p aspen-ticket --no-default-features`
- `cargo check -p aspen-ticket --target wasm32-unknown-unknown`
- `cargo check -p aspen-ticket --no-default-features --target wasm32-unknown-unknown`
- `cargo check -p aspen-ticket --features iroh`
- `cargo check -p aspen-ticket --no-default-features --features signed`
- `cargo check -p aspen-ticket --no-default-features --features signed --target wasm32-unknown-unknown`
- `cargo check -p aspen-ticket --features std`
- a dedicated default-vs-`--no-default-features` comparison artifact at `evidence/default-vs-no-default-equivalence.md`
- a dedicated unsigned wire-break artifact at `evidence/unsigned-wire-break.md` that shows current alloc-safe roundtrip success and legacy unsigned payload rejection side by side
- a checked-in legacy unsigned fixture source at `crates/aspen-ticket/tests/legacy.rs` to generate or hold the pre-change payload bytes used by `evidence/unsigned-wire-break.md`
- targeted unsigned roundtrip, invalid-input, malformed-topic rejection, lossless alloc-safe-topic ↔ `iroh_gossip::proto::TopicId` conversion, and legacy-schema rejection tests
- targeted signed-ticket positive/negative tests for explicit-time validation, malformed/corrupted signed-input rejection, and fail-loud encoding
- deterministic negative proof for the signed-only surface via a compile-fail/UI rail (for example `cargo test -p aspen-ticket --test ui`) that tries to call `sign`, `sign_with_validity`, `verify`, and `is_expired` without `std`
- deterministic negative proof for the bare/default surface via a compile-fail/UI rail that tries to call iroh-only constructors/conversions without `iroh`
- explicit positive `std` proof (for example `cargo test -p aspen-ticket --features std --test std`) for the wall-clock / nonce-generating convenience wrappers
- deterministic source audit proving the signed encoder no longer uses `unwrap_or_default()` / empty-payload fallback
- deterministic direct-consumer audit for every `aspen-ticket` dependency stanza
- exact compile rails for every direct `aspen-ticket` consumer named in the audit scope
- root workspace stanza proof showing `Cargo.toml` no longer re-enables runtime features implicitly via `evidence/workspace-dependency-proof.txt`
- synchronized `verification.md`, `implementation-diff.txt`, and `openspec-preflight.txt`

## Risks / Trade-offs

- **Topic-id migration churn:** replacing `TopicId` with an alloc-safe newtype touches many callers. Mitigate with explicit runtime conversion helpers and exhaustive direct-consumer compile rails.
- **Signed-feature split can miss a consumer:** mitigate with a direct manifest audit and compile rails for every changed direct consumer.
- **Legacy unsigned ticket decode breakage:** accepted as intentional; Aspen does not prioritize backward compatibility for this seam.
