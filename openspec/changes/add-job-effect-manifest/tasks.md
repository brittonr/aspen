## Phase 1: Effect taxonomy and mapping

- [ ] [serial] Inventory current job/service/worker capability checks, secret injection, blob/KV access, network access, and receipt redaction behavior.
- [ ] [depends:inventory] Define the versioned effect taxonomy and manifest schema, including read/write distinctions and extension rules.
- [ ] [depends:taxonomy] Define the mapping from requested effects to existing UCAN/capability grants and denial reasons.

## Phase 2: Enforcement slice

- [ ] [depends:capability-mapping] Select one executor/runtime slice and wire deny-by-default admission before runtime start.
- [ ] [depends:admission] Derive sandbox/environment policy for the selected slice from the admitted effect manifest.
- [ ] [depends:sandbox] Block or reject undeclared effect use through Aspen-controlled APIs for the selected slice.

## Phase 3: Receipts and validation

- [ ] [depends:enforcement] Emit effect-aware receipts with declared/granted/denied summaries and redacted handles.
- [ ] [depends:receipts] Add positive tests for declared/granted effects and negative tests for unknown, ungranted, and undeclared effects plus secret redaction.
- [ ] [depends:tests] Update docs and run focused effect-manifest tests, strict OpenSpec validation, and `git diff --check`.
