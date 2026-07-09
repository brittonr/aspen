## Context

Configuration review currently depends on multiple surfaces: docs describe Nickel profile contracts, runtime config parsing validates a narrow JSON export, node startup receipts bind selected refs, and Nix/Cairn checks validate repository-level config. These are useful but do not provide a single deterministic readback that explains the effective values and their sources.

## Decisions

### Effective config is a canonical artifact

**Choice:** Represent effective config readback as a canonical Preserves artifact with schema metadata, profile refs, effective values, source traces, caveats, and checks. The fingerprint is BLAKE3 over canonical bytes.

**Rationale:** Reviewers need stable identity and diffs over configuration without treating rendered text as normative.

### Source traces are explicit

**Choice:** Each effective field records whether it came from a profile, CLI override, default, environment-resolved shell input, ledger evidence, or fixture fallback.

**Rationale:** The main configurability pain is not only the final value; it is knowing whether that value was reviewed, overridden, or hidden behind a default.

### Diff and explain share the normalization core

**Choice:** `validate`, `export`, `explain`, `diff`, and `fingerprint` call the same pure normalization core. CLI code only reads files, resolves user-supplied paths, writes artifacts, and renders summaries.

**Rationale:** Diff/fingerprint behavior must be testable from in-memory inputs and independent of terminal output.

### Readbacks remain non-authoritative

**Choice:** Effective-config artifacts can be referenced by receipts and runbooks, but downstream gates must still require their normal policy, authority, source-gate, resource, provenance, retention, transport, and release evidence.

**Rationale:** Config visibility helps operators, but visibility is not permission or correctness.

## Validation strategy

- Unit tests for source trace normalization, conflict detection, canonical fingerprint stability, and readback diffs.
- Negative tests for conflicting overrides, unsupported profile metadata, stale refs, local-fixture defaults in release mode, and non-canonical fingerprint inputs.
- CLI tests that assert canonical artifacts before rendered summaries.

## Non-claims

Effective-config readbacks do not prove that a node started, that adapters are healthy, that release evidence is current, or that an operator is authorized to run a command.
