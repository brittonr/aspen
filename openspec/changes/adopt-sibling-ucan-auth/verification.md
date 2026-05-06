# Drain Verification Matrix

## I1 Aspen auth call-site inventory

- Rail: evidence-only source inventory
- Command: content search and source-anchor inspection across `crates/aspen-auth-core`, `crates/aspen-auth`, `src/bin/aspen-token.rs`, RPC handlers, federation, and service wrappers.
- Status: PASS
- Artifact: `openspec/changes/adopt-sibling-ucan-auth/evidence/i1-aspen-auth-callsite-inventory.md`
- Scope rationale: Task I1 required inventory of existing Aspen auth issuance, parsing, verification, delegation, federation credential, and RPC admission call sites; no source change was required.
- Next best check: sibling `../ucan` API inventory and adapter-boundary classification for the next task.
