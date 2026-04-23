Evidence-ID: alloc-safe-hooks-ticket.root-verification
Task-ID: V1
Artifact-Type: verification-index
Covers: architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.bare-hook-ticket-dependency-stays-alloc-safe, architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.expiry-math-stays-testable-without-wall-clock, architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-hook-tickets-roundtrip-successfully, architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.default-and-explicit-alloc-safe-surfaces-remain-equivalent, architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-dependency-edge-stays-alloc-safe, architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.runtime-conversion-happens-at-the-shell-boundary, architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.std-convenience-wrappers-require-explicit-opt-in, architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.hook-ticket-seam-proof-is-reviewable, architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.parse-and-validation-failures-use-hook-ticket-errors, architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.legacy-serialized-hook-tickets-are-rejected-explicitly, architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.runtime-consumers-surface-legacy-decode-failures-explicitly, architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.hook-ticket-error-surface-proof-is-reviewable, ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload.hook-ticket-encoder-fails-loudly-on-serializer-invariant-break, ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload.hook-ticket-encoder-fail-loud-proof-is-reviewable

# Verification Evidence

## Implementation Evidence

- Changed file: `Cargo.lock`
- Changed file: `crates/aspen-cli/Cargo.toml`
- Changed file: `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs`
- Changed file: `crates/aspen-hooks/Cargo.toml`
- Changed file: `crates/aspen-hooks/src/client.rs`
- Changed file: `crates/aspen-hooks-ticket/Cargo.toml`
- Changed file: `crates/aspen-hooks-ticket/src/lib.rs`
- Changed file: `crates/aspen-hooks-ticket/tests/legacy.rs`
- Changed file: `crates/aspen-hooks-ticket/tests/std.rs`
- Changed file: `crates/aspen-hooks-ticket/tests/ui.rs`
- Changed file: `crates/aspen-hooks-ticket/tests/ui/std_wrappers_require_feature.rs`
- Changed file: `crates/aspen-hooks-ticket/tests/ui/std_wrappers_require_feature.stderr`
- Changed file: `openspec/changes/alloc-safe-hooks-ticket/.openspec.yaml`
- Changed file: `openspec/changes/alloc-safe-hooks-ticket/design.md`
- Changed file: `openspec/changes/alloc-safe-hooks-ticket/evidence/baseline-validation.md`
- Changed file: `openspec/changes/alloc-safe-hooks-ticket/evidence/default-vs-no-default-equivalence.md`
- Changed file: `openspec/changes/alloc-safe-hooks-ticket/evidence/final-validation.md`
- Changed file: `openspec/changes/alloc-safe-hooks-ticket/evidence/implementation-diff.txt`
- Changed file: `openspec/changes/alloc-safe-hooks-ticket/evidence/openspec-preflight.txt`
- Changed file: `openspec/changes/alloc-safe-hooks-ticket/proposal.md`
- Changed file: `openspec/changes/alloc-safe-hooks-ticket/specs/architecture-modularity/spec.md`
- Changed file: `openspec/changes/alloc-safe-hooks-ticket/specs/ticket-encoding/spec.md`
- Changed file: `openspec/changes/alloc-safe-hooks-ticket/tasks.md`
- Changed file: `openspec/changes/alloc-safe-hooks-ticket/verification.md`

## Task Coverage

- [x] R1 Capture the current `aspen-hooks-ticket` dependency/compile baseline before seam edits, including full-graph `cargo tree -p aspen-hooks-ticket -e normal`, `cargo test -p aspen-hooks-ticket`, `cargo check -p aspen-hooks`, and `cargo check -p aspen-cli`, and save the reviewable output under `openspec/changes/alloc-safe-hooks-ticket/evidence/baseline-validation.md`. [covers=architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.hook-ticket-seam-proof-is-reviewable]
  - Evidence: `openspec/changes/alloc-safe-hooks-ticket/evidence/baseline-validation.md`

- [x] I1 Refactor `crates/aspen-hooks-ticket` into an alloc-safe leaf crate: add `no_std`/`alloc` scaffolding, replace public bootstrap peers with `NodeAddress`, replace `anyhow` with a crate-local error type, move expiry logic to explicit-time helpers, keep runtime wall-clock wrappers behind an explicit `std` feature, reject legacy serialized ticket payloads through the crate-local error surface using the checked-in test fixture `crates/aspen-hooks-ticket/tests/legacy.rs`, and remove silent empty-payload serialization fallback. [covers=architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.bare-hook-ticket-dependency-stays-alloc-safe,architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.expiry-math-stays-testable-without-wall-clock,architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-hook-tickets-roundtrip-successfully,architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-dependency-edge-stays-alloc-safe,architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.std-convenience-wrappers-require-explicit-opt-in,architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.parse-and-validation-failures-use-hook-ticket-errors,architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.legacy-serialized-hook-tickets-are-rejected-explicitly,architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.hook-ticket-error-surface-proof-is-reviewable,ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload.hook-ticket-encoder-fails-loudly-on-serializer-invariant-break]
  - Evidence: `crates/aspen-hooks-ticket/Cargo.toml`, `crates/aspen-hooks-ticket/src/lib.rs`, `crates/aspen-hooks-ticket/tests/legacy.rs`, `crates/aspen-hooks-ticket/tests/std.rs`, `crates/aspen-hooks-ticket/tests/ui.rs`, `crates/aspen-hooks-ticket/tests/ui/std_wrappers_require_feature.rs`, `crates/aspen-hooks-ticket/tests/ui/std_wrappers_require_feature.stderr`, `openspec/changes/alloc-safe-hooks-ticket/evidence/final-validation.md`, `openspec/changes/alloc-safe-hooks-ticket/evidence/implementation-diff.txt`

- [x] I2 Update runtime consumers (`crates/aspen-hooks`, `crates/aspen-cli`) to opt into conversion explicitly, convert `NodeAddress` bootstrap peers at the shell boundary, add positive/negative regression coverage for valid and invalid bootstrap-peer conversion paths plus legacy decode-failure surfacing, and keep the direct `aspen-cluster-types = { default-features = false, features = ["iroh"] }` manifest edge explicit in both runtime consumer `Cargo.toml` files. [covers=architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.runtime-conversion-happens-at-the-shell-boundary,architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.runtime-consumers-surface-legacy-decode-failures-explicitly]
  - Evidence: `crates/aspen-hooks/Cargo.toml`, `crates/aspen-hooks/src/client.rs`, `crates/aspen-cli/Cargo.toml`, `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs`, `openspec/changes/alloc-safe-hooks-ticket/evidence/final-validation.md`, `openspec/changes/alloc-safe-hooks-ticket/evidence/implementation-diff.txt`

- [x] I3 Add durable verification plumbing for this seam: capture alloc-safe default-vs-`--no-default-features` equivalence, keep the reviewable artifact index/current diff/preflight artifacts synchronized, record the deterministic encoder/source audit command for `AspenHookTicket::to_bytes()`, add the deterministic dependency-audit assertions that both alloc-safe surfaces exclude `iroh`, `anyhow`, `aspen-core-shell`, `aspen-hooks`, `aspen-cli`, and `tokio`, and record the deterministic manifest audit for `crates/aspen-hooks-ticket/Cargo.toml` proving the `aspen-cluster-types` edge keeps `default-features = false` with no `iroh` opt-in. [covers=architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.default-and-explicit-alloc-safe-surfaces-remain-equivalent,architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-dependency-edge-stays-alloc-safe,architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.hook-ticket-seam-proof-is-reviewable,ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload.hook-ticket-encoder-fail-loud-proof-is-reviewable]
  - Evidence: `openspec/changes/alloc-safe-hooks-ticket/evidence/default-vs-no-default-equivalence.md`, `openspec/changes/alloc-safe-hooks-ticket/evidence/final-validation.md`, `openspec/changes/alloc-safe-hooks-ticket/evidence/implementation-diff.txt`, `openspec/changes/alloc-safe-hooks-ticket/evidence/openspec-preflight.txt`

- [x] V1 Save final reviewable proof under `openspec/changes/alloc-safe-hooks-ticket/evidence/` for `cargo test -p aspen-hooks-ticket`, `cargo test -p aspen-hooks-ticket --test ui`, `cargo test -p aspen-hooks-ticket --features std --test std`, `cargo check -p aspen-hooks-ticket`, `cargo check -p aspen-hooks-ticket --target wasm32-unknown-unknown`, `cargo check -p aspen-hooks-ticket --no-default-features`, `cargo check -p aspen-hooks-ticket --no-default-features --target wasm32-unknown-unknown`, `cargo check -p aspen-hooks-ticket --features std`, full-graph `cargo tree -p aspen-hooks-ticket -e normal`, full-graph `cargo tree -p aspen-hooks-ticket -e features`, full-graph `cargo tree -p aspen-hooks-ticket --no-default-features -e normal`, full-graph `cargo tree -p aspen-hooks-ticket --no-default-features -e features`, the deterministic dependency-audit command asserting both alloc-safe surfaces exclude `iroh`, `anyhow`, `aspen-core-shell`, `aspen-hooks`, `aspen-cli`, and `tokio`, `cargo check -p aspen-hooks`, `cargo check -p aspen-cli`, `cargo tree -p aspen-hooks -e features -i aspen-cluster-types`, `cargo tree -p aspen-cli -e features -i aspen-cluster-types`, the deterministic manifest-audit command asserting `crates/aspen-hooks-ticket/Cargo.toml` keeps `aspen-cluster-types = { default-features = false }` with no `iroh` opt-in plus direct `aspen-cluster-types = { default-features = false, features = ["iroh"] }` entries in both runtime consumer manifests, and targeted positive/negative regression tests for current-schema `NodeAddress` roundtrip preservation, invalid `default_payload` JSON, expiry, legacy-ticket rejection, runtime bootstrap-peer conversion, and runtime legacy decode-failure surfacing in both consumers. Keep `verification.md`, `evidence/default-vs-no-default-equivalence.md`, `evidence/implementation-diff.txt`, and `evidence/openspec-preflight.txt` synchronized with those saved artifacts. [covers=architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.bare-hook-ticket-dependency-stays-alloc-safe,architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.expiry-math-stays-testable-without-wall-clock,architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-hook-tickets-roundtrip-successfully,architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.default-and-explicit-alloc-safe-surfaces-remain-equivalent,architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-dependency-edge-stays-alloc-safe,architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.runtime-conversion-happens-at-the-shell-boundary,architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.std-convenience-wrappers-require-explicit-opt-in,architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.hook-ticket-seam-proof-is-reviewable,architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.parse-and-validation-failures-use-hook-ticket-errors,architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.legacy-serialized-hook-tickets-are-rejected-explicitly,architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.runtime-consumers-surface-legacy-decode-failures-explicitly,architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.hook-ticket-error-surface-proof-is-reviewable,ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload.hook-ticket-encoder-fails-loudly-on-serializer-invariant-break,ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload.hook-ticket-encoder-fail-loud-proof-is-reviewable] [evidence=openspec/changes/alloc-safe-hooks-ticket/evidence/final-validation.md]
  - Evidence: `openspec/changes/alloc-safe-hooks-ticket/evidence/final-validation.md`, `openspec/changes/alloc-safe-hooks-ticket/evidence/default-vs-no-default-equivalence.md`, `openspec/changes/alloc-safe-hooks-ticket/evidence/implementation-diff.txt`, `openspec/changes/alloc-safe-hooks-ticket/evidence/openspec-preflight.txt`

## Review Scope Snapshot

### `git diff HEAD -- Cargo.lock crates/aspen-cli/Cargo.toml crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs crates/aspen-hooks/Cargo.toml crates/aspen-hooks/src/client.rs crates/aspen-hooks-ticket/Cargo.toml crates/aspen-hooks-ticket/src/lib.rs crates/aspen-hooks-ticket/tests/legacy.rs crates/aspen-hooks-ticket/tests/std.rs crates/aspen-hooks-ticket/tests/ui.rs crates/aspen-hooks-ticket/tests/ui/std_wrappers_require_feature.rs crates/aspen-hooks-ticket/tests/ui/std_wrappers_require_feature.stderr openspec/changes/alloc-safe-hooks-ticket`

- Status: captured
- Artifact: `openspec/changes/alloc-safe-hooks-ticket/evidence/implementation-diff.txt`

## Verification Commands

### `cargo tree -p aspen-hooks-ticket -e normal` baseline bundle plus pre-change compile rails

- Status: pass
- Artifact: `openspec/changes/alloc-safe-hooks-ticket/evidence/baseline-validation.md`

### `cargo test/check/tree` validation bundle for alloc-safe, std, wasm, runtime-consumer, and source-audit rails

- Status: pass
- Artifact: `openspec/changes/alloc-safe-hooks-ticket/evidence/final-validation.md`

### `cargo tree` default-vs-`--no-default-features` comparison

- Status: pass
- Artifact: `openspec/changes/alloc-safe-hooks-ticket/evidence/default-vs-no-default-equivalence.md`

### `git diff HEAD -- Cargo.lock crates/aspen-cli/Cargo.toml crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs crates/aspen-hooks/Cargo.toml crates/aspen-hooks/src/client.rs crates/aspen-hooks-ticket/Cargo.toml crates/aspen-hooks-ticket/src/lib.rs crates/aspen-hooks-ticket/tests/legacy.rs crates/aspen-hooks-ticket/tests/std.rs crates/aspen-hooks-ticket/tests/ui.rs crates/aspen-hooks-ticket/tests/ui/std_wrappers_require_feature.rs crates/aspen-hooks-ticket/tests/ui/std_wrappers_require_feature.stderr openspec/changes/alloc-safe-hooks-ticket`

- Status: captured
- Artifact: `openspec/changes/alloc-safe-hooks-ticket/evidence/implementation-diff.txt`

### `scripts/openspec-preflight.sh alloc-safe-hooks-ticket`

- Status: pass
- Artifact: `openspec/changes/alloc-safe-hooks-ticket/evidence/openspec-preflight.txt`

## Notes

- `cargo check -p aspen-cli` and `cargo test -p aspen-cli ...` emit pre-existing `unknown lint: ambient_clock` warnings in this environment; they are not introduced by this seam.
- Targeted runtime-consumer tests use `TMPDIR=/run/user/1555/tmp` because the default `/tmp` tmpfs was full during this session.
