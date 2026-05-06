# Drain Verification Matrix

## I1 Aspen auth call-site inventory

- Rail: evidence-only source inventory
- Command: content search and source-anchor inspection across `crates/aspen-auth-core`, `crates/aspen-auth`, `src/bin/aspen-token.rs`, RPC handlers, federation, and service wrappers.
- Status: PASS
- Artifact: `openspec/changes/adopt-sibling-ucan-auth/evidence/i1-aspen-auth-callsite-inventory.md`
- Scope rationale: Task I1 required inventory of existing Aspen auth issuance, parsing, verification, delegation, federation credential, and RPC admission call sites; no source change was required.
- Next best check: sibling `../ucan` API inventory and adapter-boundary classification for the next task.

## I2 sibling UCAN public API inventory

- Rail: evidence-only source inventory and ownership classification
- Command: source-anchor inspection of sibling `../ucan` no-std core, root runtime shell, token workflow, readiness report, public API docs, and integration examples.
- Status: PASS
- Artifact: `openspec/changes/adopt-sibling-ucan-auth/evidence/i2-sibling-ucan-api-inventory.md`
- Scope rationale: Task I2 required classifying sibling public APIs between portable `aspen-auth-core`, runtime `aspen-auth`, and Aspen-only adapter surfaces; no Rust source change was required.
- Boundary conclusion: `aspen-auth-core` should depend only on `ucan-core` no-std validation/claim-shape APIs; `aspen-auth` should own root `ucan` compact-token issuance, verification, resolver/proof/revocation/replay/caveat hooks, and shell IO/readiness adapters.
- Next best check: Aspen capability/operation to UCAN resource/ability mapping table, including unsupported or intentionally Aspen-local capabilities.

## I3 Aspen capability/operation to UCAN mapping

- Rail: evidence-only mapping table and adapter decision record
- Command: source-anchor inspection of `Capability`, `Operation`, `authorizes`, `contains`, and sibling UCAN resource-prefix / ability wildcard behavior.
- Status: PASS
- Artifact: `openspec/changes/adopt-sibling-ucan-auth/evidence/i3-aspen-ucan-capability-mapping.md`
- Scope rationale: Task I3 required a mapping table, including unsupported or Aspen-local semantics; no Rust adapter implementation was required.
- Boundary conclusion: map Aspen resources as `aspen:<domain>:<scope>` and abilities as `aspen/<domain>/<verb>`; keep shell globs, admin implication sets, delegate issuance gate, batch all-item checks, and audit-only fields in the Aspen adapter.
- Next best check: controlled Cargo/Nix dependency wiring for sibling `../ucan` and protected dependency graph evidence.

## I4 controlled UCAN dependency wiring

- Rail: Cargo/Nix dependency wiring and local-development policy
- Status: PASS
- Artifact: `openspec/changes/adopt-sibling-ucan-auth/evidence/i4-controlled-ucan-dependency-wiring.md`
- Commands: `cargo metadata`, `cargo check -p aspen-auth-core --no-default-features`, `cargo check -p aspen-auth --all-targets`, `nix flake lock`, `nix flake metadata`, and locked `cargo metadata`.
- Boundary conclusion: `aspen-auth-core` is wired only to `ucan-core`; `aspen-auth` is wired to root `ucan`; local `../ucan` path patching is opt-in and commented; Nix uses a locked `ucan-src` source override.
- Known failure mode: private SSH fetch requires GitHub credentials until UCAN is public or mirrored into Aspen-owned cache/source distribution.
- Next best check: prove dependency boundaries with `cargo tree`/feature checks for `aspen-auth-core`, `aspen-auth`, and protected `aspen-core --no-default-features` paths.

## I5 UCAN dependency-boundary proof

- Rail: `cargo tree`/feature boundary evidence plus deterministic protected-core checker
- Status: PASS
- Artifact: `openspec/changes/adopt-sibling-ucan-auth/evidence/i5-ucan-dependency-boundary-proof.md`
- Commands: `cargo tree -p aspen-auth-core --no-default-features`, `cargo tree -p aspen-auth`, `cargo tree -p aspen-core --no-default-features`, feature-tree variants, and `scripts/check-aspen-core-no-std-boundary.py`.
- Boundary conclusion: `aspen-auth-core` includes only `ucan-core`; `aspen-auth` includes root `ucan` and `verified-logic`; protected `aspen-core --no-default-features` excludes Aspen auth and all UCAN dependencies.
- Next best check: implement the UCAN-backed adapter while preserving Aspen-facing capability/token/RPC/CLI behavior.

## I6 UCAN adapter implementation

- Rail: runtime adapter implementation without changing legacy token/RPC/CLI behavior yet.
- Status: PASS
- Artifact: `openspec/changes/adopt-sibling-ucan-auth/evidence/i6-ucan-adapter-implementation.md`
- Code: `crates/aspen-auth/src/ucan_adapter.rs`; exported by `crates/aspen-auth/src/lib.rs`.
- Commands: `nix run .#rustfmt`; `CARGO_TARGET_DIR=target/agent cargo test -p aspen-auth ucan_adapter --all-targets`.
- Boundary conclusion: Aspen `Capability` variants now project to sibling-validated UCAN capability documents and sets; legacy token wire format and admission paths remain unchanged pending compatibility/negative evidence.
- Next best check: add compatibility fixtures for existing Aspen token generation/inspection/delegation behavior or document intentional migration receipts.

## I7 legacy Aspen token compatibility fixtures

- Rail: compatibility fixtures before runtime verifier/admission switch.
- Status: PASS
- Artifact: `openspec/changes/adopt-sibling-ucan-auth/evidence/i7-legacy-token-compatibility-fixtures.md`
- Code: `crates/aspen-auth/src/tests.rs`
- Commands: `rustfmt crates/aspen-auth/src/tests.rs`; `CARGO_TARGET_DIR=target/agent cargo test -p aspen-auth adopt_sibling_ucan_compat_fixture --all-targets`.
- Boundary conclusion: legacy Aspen base64 token roundtrip, delegation proof/depth shape, and debug redaction receipts remain preserved; no intentional token-format migration happened in this slice.
- Next best check: add negative evidence, then switch runtime verification/admission paths only where compatibility remains proven.

## I8 runtime verifier UCAN adapter switch

- Rail: runtime verification/admission boundary.
- Status: PASS
- Artifact: `openspec/changes/adopt-sibling-ucan-auth/evidence/i8-runtime-verifier-ucan-adapter-switch.md`
- Code: `crates/aspen-auth/src/verifier.rs`
- Commands: `rustfmt crates/aspen-auth/src/verifier.rs`; `CARGO_TARGET_DIR=target/agent cargo test -p aspen-auth test_verifier --all-targets`.
- Boundary conclusion: direct and delegation-chain token verification now require Aspen capabilities to project into sibling-validated UCAN capability documents before runtime admission proceeds; legacy token wire format and Aspen operation authorization semantics remain preserved.
- Next best check: run final OpenSpec validation/docs/focused test suite and capture release notes.

## I9 UCAN/Aspen positive and negative tests

- Rail: focused compatibility/negative evidence before final docs.
- Status: PASS
- Artifact: `openspec/changes/adopt-sibling-ucan-auth/evidence/i9-ucan-aspen-positive-negative-tests.md`
- Code: `crates/aspen-auth/src/ucan_adapter.rs`
- Commands: `rustfmt crates/aspen-auth/src/ucan_adapter.rs`; `CARGO_TARGET_DIR=target/agent cargo test -p aspen-auth ucan_adapter --all-targets`; `CARGO_TARGET_DIR=target/agent cargo test -p aspen-auth test_verifier --all-targets`.
- Boundary conclusion: positive UCAN projection tests and denied empty UCAN capability-set mapping are retained; existing verifier negatives cover signature, expiry, audience, revocation, and proof-chain failures.
- Next best check: update auth/federation/operator docs with adapter boundary and migration caveats.
