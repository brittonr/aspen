## Why

`crates/aspen-ticket` still sits on Aspen's runtime side of the no-std/std boundary. Its default surface pulls `iroh`, `iroh-gossip`, `rand`, `anyhow`, and ambient wall-clock helpers into a shared ticket crate that many consumers only need for parsing or holding cluster bootstrap data. It also still exposes concrete iroh transport types in the unsigned ticket payload and still uses a silent empty-payload fallback in the signed ticket encoder.

That makes `aspen-ticket` the next same-family seam after `aspen-cluster-types` and `aspen-hooks-ticket`: cluster bootstrap metadata should stay transport-neutral and alloc-safe by default, while runtime helpers and signed-ticket conveniences should be explicit opt-ins.

## What Changes

- Make `crates/aspen-ticket` alloc-safe by default around the unsigned cluster ticket core.
- **BREAKING** Replace the unsigned ticket's public runtime transport metadata with alloc-safe types: `aspen_cluster_types::NodeAddress` bootstrap peers reached through `aspen-cluster-types = { default-features = false }`, plus a crate-local alloc-safe topic identifier instead of concrete iroh gossip/runtime types. Previously serialized unsigned cluster tickets will need regeneration after this schema change.
- Replace `anyhow`-style parse/validation APIs with crate-local ticket error types for the unsigned alloc-safe surface.
- Move runtime-only conversion helpers behind explicit Cargo features: `iroh` for unsigned transport/gossip conversions, `signed` for explicit-time signed-ticket types and verification helpers, and `std` as the convenience bundle that implies `signed` and adds only wall-clock / nonce-generating signed helpers.
- Remove the silent empty-payload fallback from the signed cluster ticket encoder so impossible serializer failures fail loudly.
- Update every direct runtime consumer whose manifest classification changes so local feature opt-in and shell-boundary conversion stay reviewable.

## Non-Goals

- Refactoring `crates/aspen-hooks-ticket` again in this change.
- Redesigning cluster bootstrap semantics, gossip behavior, or signed-ticket trust rules beyond the feature-boundary and payload-shape changes above.
- Preserving wire compatibility with pre-change unsigned ticket payloads.
- Removing runtime signing/verification support entirely; this change makes it explicit, not absent.

## Capabilities

### Modified Capabilities
- `architecture-modularity`: extend the alloc-safe leaf-crate boundary so `aspen-ticket` defaults to transport-neutral unsigned ticket data and only runtime consumers opt into iroh or signed-ticket helpers.
- `ticket-encoding`: require signed cluster ticket encoding to fail loudly instead of substituting empty payload bytes.

## Impact

- **Files**: `crates/aspen-ticket/`, direct runtime consumers of `aspen-ticket`, and OpenSpec artifacts under `openspec/changes/alloc-safe-cluster-ticket/`.
- **Direct consumers to audit**: exhaustive set discovered from a workspace-graph query over `cargo metadata --format-version 1 --no-deps` (not a single manifest regex): root `Cargo.toml`, `crates/aspen-ci-executor-vm`, `crates/aspen-cluster-handler`, `crates/aspen-cluster`, `crates/aspen-rpc-handlers`, `crates/aspen-client`, and `crates/aspen-ci`.

| Consumer | Planned feature surface | Manifest changes? | Planned compile / audit rail |
| --- | --- | --- | --- |
| `Cargo.toml` workspace stanza | explicit alloc-safe workspace stanza (`default-features = false`) | yes | `rg -n '^aspen-ticket\\s*=\\s*\\{' Cargo.toml` |
| `crates/aspen-ci-executor-vm` | bare/default | no local stanza change expected | `cargo check -p aspen-ci-executor-vm` |
| `crates/aspen-cluster-handler` | `iroh` | yes | `cargo check -p aspen-cluster-handler --tests` |
| `crates/aspen-cluster` | `iroh`, `std` | yes | `cargo check -p aspen-cluster` |
| `crates/aspen-rpc-handlers` | bare/default | no local stanza change expected | `cargo check -p aspen-rpc-handlers` |
| `crates/aspen-client` | `iroh` | yes | `cargo check -p aspen-client` |
| `crates/aspen-ci` | bare/default | no local stanza change expected | `cargo check -p aspen-ci` |

- **APIs**: unsigned ticket bootstrap metadata and topic identifiers become alloc-safe; runtime conversion and signed-ticket helpers become explicit feature-gated opt-ins.
- **Dependencies**: default `aspen-ticket` resolution should stop pulling `iroh`, `iroh-gossip`, `rand`, and `anyhow`; runtime consumers add explicit opt-ins where needed.
- **Legacy unsigned specimen source**: save a checked-in pre-change unsigned ticket generator/fixture at `crates/aspen-ticket/tests/legacy.rs` so `evidence/unsigned-wire-break.md` can show both current alloc-safe roundtrip success and explicit rejection of the old runtime payload layout.
- **Testing**: targeted full-graph `cargo tree`, `cargo check`, and exact compile rails for `cargo check -p aspen-ticket --target wasm32-unknown-unknown`, `cargo check -p aspen-ticket --no-default-features --target wasm32-unknown-unknown`, and `cargo check -p aspen-ticket --no-default-features --features signed --target wasm32-unknown-unknown`; deterministic source audit of `SignedAspenClusterTicket::to_bytes()` fallback removal; ticket roundtrip/regression rails; shell-boundary conversion-site citations for every `iroh` consumer; representative transitive re-export leak proofs beyond direct consumers; and consumer compile proofs for each audited consumer path. Save distinct evidence for current alloc-safe unsigned roundtrip success, legacy unsigned payload rejection, default-vs-`--no-default-features` equivalence, and signed malformed-input rejection.
