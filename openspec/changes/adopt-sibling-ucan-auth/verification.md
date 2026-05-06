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
