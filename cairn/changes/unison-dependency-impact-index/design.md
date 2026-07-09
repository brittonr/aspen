## Context

Artifact refs are only useful when Molten can explain the graph around them. Dependency information should be stored as canonical metadata produced by artifact canonicalizers and admission paths, not inferred only by scanning opaque bytes.

## Design

### Edge model

Dependency edges are canonical records:

```text
dependency-edge-v1
  from: artifact/ref
  to: artifact | schema | policy | effect | capability | handler-profile | storage-record | transcript | release-snapshot
  relation: imports | validates-with | stores-as | migrates-with | invokes | documents | expects | packages | supersedes
  required: true | false
  scope and evidence refs
```

Edges are direct facts. Transitive closures and reverse dependents are computed indexes with receipts.

### Impact queries

Impact query receipts bind:

- query subject and relation filters;
- direct dependents;
- transitive dependents when requested;
- redaction decisions;
- index version or rebuild receipt;
- stale/missing/duplicate diagnostics.

Upgrade sessions, retention plans, release snapshots, and catalog tools consume these receipts as planning evidence. They do not replace the downstream gate that decides whether a mutation is safe.

### Rebuild determinism

Given the same registry artifacts and ledger edge records, a rebuild must produce the same sorted edge set, reverse index, and index digest. Stale indexes deny normative impact gates until rebuilt or explicitly marked diagnostic-only.

### Functional core and shell

Pure cores normalize edge records, compute reverse indexes, detect cycles and duplicates, and evaluate query filters from in-memory inputs. Shells read registry/ledger state, persist indexes, enforce redaction policy, and render diagnostics.

### Non-goals

- Do not infer hidden dependency trust from display names or source text.
- Do not expose redacted/private dependency targets through public catalog views.
- Do not let impact query pass evidence authorize mutation, deletion, execution, or release by itself.
- Do not adopt UCM codebase internals or dependency formats.