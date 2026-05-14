## Context

Aspen already has several receipt-like evidence surfaces:

- dogfood run receipts for the self-hosting loop;
- CI run receipts and artifact metadata;
- runtime-host product-path logs and markers;
- job outputs and worker diagnostics.

These surfaces should converge on shared operator semantics without forcing one schema to replace all existing receipts at once.

## Design

### Receipt model

A runtime application receipt should identify:

- schema and version;
- receipt id and parent run/job/service id when any;
- source command or API operation;
- runtime unit or service name;
- host kind and runner identity;
- artifact identity and provenance handles;
- lifecycle status and timestamps/durations;
- bounded output/artifact references;
- redacted failure category and diagnostics when failed.

### Validation

When a Rust type owns canonical serialization, Aspen should generate a typed Nickel contract and require freshness checks. Legacy receipts remain readable by schema-specific adapters.

### Readback

Operators should be able to list/show/diagnose receipts without scraping logs. Readback may start with CLI/API commands over local receipt stores or Raft-backed KV; the contract is that output is validated, bounded, and secret-safe.

### Compatibility

Dogfood and CI receipts remain valid. The generalized receipt model should share fields and rendering behavior where practical, not force a flag-day migration.

## Risks

- **Schema sprawl**: mitigated by shared required fields and generated contracts.
- **Log bloat**: mitigated by bounded output handles rather than raw unbounded logs.
- **Secret exposure**: mitigated by redaction requirements and fail-closed publication rules.
- **Over-migration**: mitigated by preserving existing dogfood/CI schemas and adding adapters incrementally.
