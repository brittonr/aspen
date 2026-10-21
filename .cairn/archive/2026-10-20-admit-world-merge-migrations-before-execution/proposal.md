# Proposal: Admit world merge migrations before execution

## Why

Audit finding F06 identifies a shell-to-core admission bypass at revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.
`prepare_world_merge_plan` executes a migration before the core checks its binding.
The shell then replaces source schema metadata with the target schema.
The core accepts equal schemas before it checks `admitted`.

This is source-level evidence, not an executed migration exploit. A rejecting adapter can block it, but the application contract cannot depend on that behavior.

## What Changes

- Admit the exact original source, target, profile, and migration binding before adapter execution. r[molten.audit_f06.admission]
- Bind materialization results to the admitted request without erasing original schema evidence. r[molten.audit_f06.binding]
- Deny malformed or stale results before output publication. r[molten.audit_f06.publication]
- Add core and shell positive and negative regression cases. r[molten.audit_f06.validation]

## Impact

The current consumer is `prepare_world_merge_plan` and its callers in the world-merge shell.
The Molten world-merge maintainers own this change and its regression corpus.
The durable capability is admission-first schema conversion with explicit result bindings.

Affected paths:
- `src/world_merge/service.rs:46-49,178-191`
- `src/world_merge/ports.rs`
- `crates/molten-core/src/world_merge/admission.rs:203-214`
- `.cairn/specs/world-commit/spec.md`

## Scope and Non-goals

This package plans a correction only. Implementation, publication, and lifecycle completion remain future work.
It does not change migration semantics, grant merge authority, or prove adapter correctness.
It does not mandate a new dependency or move schema policy into an adapter.

## Dependencies

The existing world-merge core and published schema-migration contract remain the integration points.
Coordinate the output representation with `distinguish-absent-world-merge-roots`, but neither package requires a circular dependency.
