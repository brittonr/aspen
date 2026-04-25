## Context

`aspen-hooks-ticket` is small, but today it sits on the runtime side of the boundary for three separate reasons:

1. `AspenHookTicket.bootstrap_peers` stores `iroh::EndpointAddr` directly.
2. expiry helpers call `SystemTime::now()` inside the ticket crate.
3. parsing/validation APIs use `anyhow` and `postcard` std paths.

Only two direct consumers currently need the ticket at runtime: `crates/aspen-hooks` and `crates/aspen-cli`. That makes this seam cheaper than `aspen-ticket` while still removing a real transport/runtime leak from a shared ticket crate.

## Goals / Non-Goals

**Goals:**
- Make `crates/aspen-hooks-ticket` compile alloc-safe by default.
- Keep shared ticket payloads transport-neutral by storing `NodeAddress` instead of `iroh::EndpointAddr`.
- Split pure expiry/validation logic from wall-clock convenience wrappers.
- Preserve current runtime behavior in `aspen-hooks` and `aspen-cli` with explicit conversion at the shell boundary.
- Save reviewable dependency/compile/test evidence for the seam.

**Non-Goals:**
- Refactoring `crates/aspen-ticket` in the same change.
- Removing iroh from runtime consumers such as `aspen-hooks` or `aspen-cli`.
- Redesigning hook-trigger RPC semantics, authentication, or payload formats.

## Decisions

### 1. Store transport-neutral bootstrap peers as `NodeAddress`

**Choice:** `AspenHookTicket.bootstrap_peers` will use `Vec<aspen_cluster_types::NodeAddress>`.

**Rationale:** `NodeAddress` already captures endpoint id plus transport addresses in an alloc-safe form and has the exact optional iroh conversion seam this crate needs.

**Alternative considered:** define another ticket-local string/socket wrapper. Rejected because Aspen already has a transport-neutral address type and duplicating it would create parallel conversion code.

### 2. Make expiry logic pure and keep wall-clock wrappers behind `std`

**Choice:** core helpers will accept `now_secs` explicitly (`with_expiry_from_now`, `is_expired_at`, `expiry_string_at`, `deserialize_at`), while runtime wrappers (`with_expiry_hours`, `is_expired`, `expiry_string`, `deserialize`) stay behind an explicit `std` feature and item-level `#[cfg(feature = "std")]` gating on those wrapper methods.

**Rationale:** this follows FCIS: the pure ticket core stays deterministic and alloc-safe, while the shell owns ambient time.

**Alternative considered:** keep ambient clock reads in the shared crate. Rejected because it keeps a leaf ticket crate on the std side of the boundary.

### 3. Use a crate-local ticket error type

**Choice:** replace `anyhow::Result` APIs with a dedicated `HookTicketError` enum.

**Rationale:** alloc-safe shared crates should not require `anyhow` just to report bounded validation and parse errors. A local error also makes negative tests more precise.

**Minimum error categories:** the enum will keep explicit variants for malformed ticket decode, invalid cluster/event/payload bounds, invalid default-payload JSON, and expired-ticket validation.

**Alternative considered:** keep `anyhow` and merely change the address type. Rejected because it would leave the crate std-bound and fail the seam's purpose.

### 4. Pin the default dependency/feature boundary explicitly

**Choice:** `crates/aspen-hooks-ticket` will use manifest-level alloc-safe constraints: `aspen-cluster-types = { default-features = false }`, `serde = { default-features = false, features = ["derive"] }`, `serde_json = { default-features = false, features = ["alloc"] }`, `postcard = { default-features = false, features = ["alloc"] }`, and `thiserror = { default-features = false }`. The crate feature map will be `default = []` with an explicit `std` feature for wall-clock wrappers and any std-only convenience glue. Runtime crates that need iroh conversion will add their own direct `aspen-cluster-types = { default-features = false, features = ["iroh"] }` dependency instead of inheriting it through the ticket crate.

**Rationale:** this is the least-risk way to reuse `NodeAddress` without recreating the earlier transitive-default leak from `aspen-cluster-types`.

**Alternative considered:** let `aspen-hooks-ticket` enable `aspen-cluster-types/iroh` or add its own `iroh` feature. Rejected because the shared crate would still be responsible for runtime transport wiring.

### 5. Runtime crates opt into conversion explicitly

**Choice:** `crates/aspen-hooks` and `crates/aspen-cli` convert `NodeAddress` values at the shell edge, each through a direct `aspen-cluster-types = { default-features = false, features = ["iroh"] }` dependency, and surface invalid stored bootstrap peers as contextual runtime errors.

**Rationale:** only runtime crates need iroh-native addresses. Keeping that conversion outside `aspen-hooks-ticket` preserves the leaf boundary and makes failures attributable.

**Alternative considered:** add conversion helpers back into `aspen-hooks-ticket`. Rejected because that would keep the shared ticket crate responsible for runtime transport glue.

### 6. Keep ticket serialization fail-loud, not fail-empty

**Choice:** `Ticket::to_bytes()` for `AspenHookTicket` will encode with alloc-safe postcard APIs and use an impossible-bug `expect`/panic path with contextual text rather than returning `Vec::new()`.

**Rationale:** the `Ticket` trait cannot return `Result`, so the correct fail-loud contract is a loud invariant break, not a silent payload substitution.

**Alternative considered:** preserve the current empty-vector fallback. Rejected because it can mint misleading tickets from invalid payload bytes.

### 7. Reject legacy serialized tickets deterministically

**Choice:** old hook tickets serialized with the `EndpointAddr` payload layout will be treated as intentionally incompatible. Parsing them will fail through `HookTicketError::Deserialize`, and runtime callers will regenerate tickets from current cluster data instead of attempting mixed-schema compatibility.

**Rationale:** backwards compatibility is not a project goal here, and a hard decode failure is simpler and safer than a dual-schema parser in a small shared crate.

**Alternative considered:** support both schemas indefinitely. Rejected because it would keep iroh-era payload assumptions alive in the alloc-safe ticket crate and complicate validation.

### 8. Save exact verification rails

**Choice:** save the following exact rails under `openspec/changes/archive/2026-04-25-alloc-safe-hooks-ticket/evidence/final-validation.md`, plus `verification.md`, `evidence/implementation-diff.txt`, and `evidence/openspec-preflight.txt`:

- `cargo test -p aspen-hooks-ticket`
- `cargo test -p aspen-hooks-ticket --test ui`
- `cargo check -p aspen-hooks-ticket`
- `cargo check -p aspen-hooks-ticket --target wasm32-unknown-unknown`
- `cargo check -p aspen-hooks-ticket --no-default-features`
- `cargo check -p aspen-hooks-ticket --no-default-features --target wasm32-unknown-unknown`
- `cargo check -p aspen-hooks-ticket --features std`
- full-graph `cargo tree -p aspen-hooks-ticket -e normal`
- full-graph `cargo tree -p aspen-hooks-ticket -e features`
- full-graph `cargo tree -p aspen-hooks-ticket --no-default-features -e normal`
- full-graph `cargo tree -p aspen-hooks-ticket --no-default-features -e features`
- a deterministic dependency-audit command recorded in `evidence/final-validation.md` that asserts both the default and `--no-default-features` `aspen-hooks-ticket` trees exclude `iroh`, `anyhow`, and named runtime shell crates such as `aspen-core-shell`, `aspen-hooks`, `aspen-cli`, and `tokio`
- a saved comparison artifact at `evidence/default-vs-no-default-equivalence.md` showing the default and `--no-default-features` alloc-safe surfaces resolve equivalently
- `cargo check -p aspen-hooks`
- `cargo check -p aspen-cli`
- `cargo tree -p aspen-hooks -e features -i aspen-cluster-types`
- `cargo tree -p aspen-cli -e features -i aspen-cluster-types`
- a deterministic manifest-audit command recorded in `evidence/final-validation.md` that asserts `crates/aspen-hooks/Cargo.toml` and `crates/aspen-cli/Cargo.toml` each declare a direct `aspen-cluster-types` dependency with `features = ["iroh"]`
- targeted tests for current-schema `NodeAddress` roundtrip preservation, invalid `default_payload` JSON, explicit-time expiry helpers, expired-ticket rejection, and legacy-ticket rejection in `aspen-hooks-ticket`
- targeted positive and negative runtime bootstrap-peer conversion tests in both runtime consumers (`aspen-hooks` and `aspen-cli`)
- targeted runtime legacy-decode failure tests in both `aspen-hooks` and `aspen-cli`
- deterministic negative proof for the `std` gate via `cargo test -p aspen-hooks-ticket --test ui`, which must attempt `with_expiry_hours`, `is_expired`, `expiry_string`, and `deserialize` without `std`
- explicit positive `std` proof via `cargo test -p aspen-hooks-ticket --features std --test std`, exercising `with_expiry_hours`, `is_expired`, `expiry_string`, and `deserialize`
- legacy-ticket regression input generated by the checked-in test-only fixture `crates/aspen-hooks-ticket/tests/legacy.rs`, which serializes the pre-change `EndpointAddr` layout outside the library surface
- deterministic encoder source audit recorded in `evidence/final-validation.md` via:
  - `python3 - <<'PY'`
  - `from pathlib import Path`
  - `text = Path("crates/aspen-hooks-ticket/src/lib.rs").read_text()`
  - `start = text.index("impl Ticket for AspenHookTicket")`
  - `end = text.index("fn validate_ticket_fields", start)`
  - `block = text[start:end]`
  - `assert "expect(" in block`
  - `assert "Vec::new()" not in block`
  - `print("ok: AspenHookTicket::to_bytes uses expect and no Vec::new fallback")`
  - `PY`
- `evidence/implementation-diff.txt` reviewed alongside `crates/aspen-hooks-ticket/src/lib.rs` to prove the encoder no longer falls back to `Vec::new()`

**Verification mapping:** `verification.md` will list every checked task verbatim and, under each task, cite the exact artifact paths that satisfy the covered requirement/scenario IDs from both delta specs.

**Rationale:** the spec requires reviewable proof that the default and `--no-default-features` alloc-safe surfaces exclude both `iroh` and `anyhow`, that alloc-safe compilation works, that the `std` gate is real, that legacy tickets fail in the documented way, that the encoder fails loudly instead of returning empty bytes, that the default and `--no-default-features` alloc-safe surfaces remain equivalent, and that both runtime consumers convert bootstrap peers through direct opt-in paths.

## Risks / Trade-offs

- **Runtime consumer churn** → `aspen-hooks` and `aspen-cli` must convert `NodeAddress` explicitly. Mitigation: keep the runtime conversion localized to the existing connection paths.
- **API break for direct ticket constructors** → callers that build `AspenHookTicket::new(..., Vec<EndpointAddr>)` must convert to `NodeAddress` first. Mitigation: only two direct runtime consumers currently do this, and both already own iroh setup code.
- **Feature split complexity** → `std` wrappers and pure helpers can drift. Mitigation: add positive and negative regression tests around explicit-time helpers and std convenience paths.
