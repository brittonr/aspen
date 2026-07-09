# Design: replay-coverage-readiness

## Scope

This change covers canonical readback of replay coverage and readiness across Molten's evidence-bearing subsystems. It does not make summaries authoritative for replay verification, release promotion, source gates, authority, provenance, policy, transport, or execution.

## Proof checklist

- **Proof claim**: replay readiness reports accurately summarize supplied replay evidence refs, identify missing or diagnostic-only coverage, and deny readiness when required positive or negative evidence is absent.
- **Out of scope**: replacing individual replay receipts, replacing subsystem gates, or proving live transport correctness.
- **Trusted assumptions**: supplied replay receipts and subsystem gate refs are parsed and content-ref validated before summarization.
- **Positive evidence**: each covered subsystem has a positive replay receipt, a negative tamper or exclusion case, and a declared replay eligibility status.
- **Negative evidence**: missing positive evidence, missing negative evidence, stale refs, diagnostic-only evidence presented as deterministic pass evidence, and duplicate subsystem entries deny matrix readiness.
- **Canonical refs**: coverage matrix ref, subsystem replay receipt refs, negative evidence refs, release replay index refs, readiness receipt refs, and caveat refs.

## Coverage model

Each subsystem row is canonical data:

```text
ReplayCoverageRow {
  subsystem
  workflow
  eligibility: deterministic | recorded | diagnostic-only | non-replayable
  fresh_run_ref?
  replay_verify_ref?
  second_fresh_run_ref?
  negative_evidence_ref?
  replay_index_ref?
  caveat_refs
}
```

The pure core validates rows, checks uniqueness by subsystem/workflow, verifies required evidence classes for eligible rows, and produces a deterministic matrix decision.

## Subsystem starting set

The first matrix should include representative paths that already have nearby evidence infrastructure:

```text
harness report replay
node-control workflow bundle
job worker scheduling and lease replay
coordination duplicate operation replay
remote dataspace delivery log replay
vat replay fixture
retention remote-clearance workflow
local dogfood release replay index
```

Additional rows can be appended without changing the matrix schema.

## Readiness shell

CLI or dogfood shell code gathers known receipts, calls the pure matrix core, writes a canonical `replay-coverage-matrix-v1` artifact and optional readiness receipt, then renders an operator summary. Catalog/MCP readback can search the matrix by subsystem, eligibility, decision, or missing-evidence diagnostic.

## Non-goals

- No broad production claim about live WAN or soak behavior.
- No promotion of diagnostic-only evidence into deterministic pass evidence.
- No replacement for replay verify receipts, rollups, indexes, subsystem gates, or release gates.
