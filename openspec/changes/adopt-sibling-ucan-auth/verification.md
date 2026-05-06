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
