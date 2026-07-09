## Context

Modules such as `node/daemon.rs`, `plugin/host.rs`, and `preserves/rail.rs` currently act as concatenation points for numbered body shards. The code may already be physically split, but the split is not a semantic boundary: every included file shares one module namespace and reviewers must infer where concepts begin and end.

## Design

### Semantic module taxonomy

Refactors should name modules after domain responsibilities, for example:

- `model` for pure data types and constants;
- `codec` for canonical boundary conversion;
- `admission` for pure pass/deny decisions;
- `receipts` for evidence value construction and parsing;
- `store` for filesystem or ledger persistence shells;
- `runner` or `service` for orchestration shells;
- `tests` for positive and negative fixtures tied to the boundary.

The exact names may vary by subsystem, but the names must express purpose rather than sequence.

### Compatibility boundary

The first migration path keeps existing public module paths in place. Existing files may re-export named submodules while callers are moved inward-facing first. Public API removal is out of scope unless another change explicitly owns that break.

### Functional core / shell split

During each semantic split, pure decision logic should move into deterministic modules that take in-memory inputs and return structured outputs. File IO, process calls, clocks, network transport, and CLI rendering remain in shell modules.

### Exemptions

Generated code or intentionally machine-partitioned fixtures may keep ordinal shards temporarily, but the owning module must record why the shard is generated or staged and what review surface remains stable.

## Non-goals

- Do not rename public APIs solely for aesthetics.
- Do not change canonical Preserves schemas or receipt refs.
- Do not split into new crates in this change; this change prepares that work.
