## Context

Molten already states that artifact names are metadata, not identity. This change makes that model concrete enough for operator UX and safe automation.

The goal is a namespace/view layer that gives humans friendly names while preserving exact BLAKE3 artifact refs in every normative artifact, receipt, dependency edge, and execution request.

## Design

### View records

Name views are canonical metadata assertions:

```text
artifact-name-view-v1
  subject: name | alias | tag | channel
  target: artifact-ref | artifact-set-ref
  scope: project | node | peer | operator | release
  issuer: capability/ref
  policy/evidence refs
  previous-view ref or tombstone ref
```

A view update emits a new metadata receipt. The target artifact is immutable and unchanged.

### Resolution and pinning

Resolution is a two-step process:

```text
name query -> candidate view records -> exact artifact ref(s) -> caller pins exact ref
```

Any artifact that can affect execution, storage, policy, replay, or release evidence must record the exact resolved ref. Rendered names may appear as diagnostics only.

### Ambiguity

If multiple candidates match and no deterministic scope or channel policy selects exactly one target, Molten denies resolution for normative use. Catalog search may display all candidates, but execution, install, migration, transcript, and policy gates require exact refs.

### Functional core and shell

Pure cores validate view records, resolve candidates from in-memory indexes, detect ambiguity, and compute diagnostics. Shells persist view receipts, enforce capability/policy updates, read local indexes, and render catalog output.

### Non-goals

- Do not adopt UCM namespaces, branch semantics, or update syntax.
- Do not make names authority, provenance, source-gate evidence, or execution admission.
- Do not allow global mutable names to rewrite existing dependency identity.
- Do not hide ambiguity behind arbitrary first-match behavior.