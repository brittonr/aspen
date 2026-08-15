## Tasks

- [x] [serial] r[aspen.ast_grep_runtime_authority_audits.profile] Define ast-grep runtime-authority audit profiles and scan scopes.
- [x] [depends:ast-grep-runtime-authority-profile] r[aspen.ast_grep_runtime_authority_audits.inventory] Add inventory-first rules for ambient authority, unsafe blocks, panics, plugin loading, and direct authority bypass candidates.
- [x] [depends:ast-grep-runtime-authority-profile] r[aspen.ast_grep_runtime_authority_audits.identity] Bind ast-grep version, rule bundle BLAKE3 identity, scan scope, runtime/evidence-gate identity, findings, and non-claims into receipts.
- [x] [depends:ast-grep-runtime-authority-inventory] r[aspen.ast_grep_runtime_authority_audits.fixtures] Add positive and negative fixtures before any audit rule becomes warning or blocking.
- [x] [depends:ast-grep-runtime-authority-fixtures] r[aspen.ast_grep_runtime_authority_audits.evidence_gates] Report findings through evidence-gate receipts without authority, replay, or release overclaims.
- [x] [depends:ast-grep-runtime-authority-evidence-gates] r[aspen.ast_grep_runtime_authority_audits.validation] Run ast-grep rule tests, authority fixture scans, Cairn gates, and focused Aspen/Molten validation rails.
