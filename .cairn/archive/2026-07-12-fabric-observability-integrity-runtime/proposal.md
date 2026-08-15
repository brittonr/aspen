## Why

Molten emits extensive canonical evidence and diagnostic fixtures, but live adapters and system extensions need one bounded observability and integrity contract. Metrics exporters, tracing, disk inspection, and operator workflows must not define extension semantics, leak unbounded labels or secrets, or turn telemetry into authority.

The fabric needs pure observation and integrity models, effectful collection/export adapters, and optional operator workflows that preserve the evidence claim boundary.

## What Changes

- Add canonical metric, event, health, readiness, integrity-plan, integrity-result, and adapter-status models with bounded dimensions and redaction classes.
- Keep aggregation, label validation, health transitions, integrity predicates, and repair recommendations in pure cores.
- Add tracing, Prometheus, OpenTelemetry, durable-state scan, content verification, and deterministic simulation adapters.
- Keep integrity checking read-only by default; repair, deletion, quarantine, or state replacement requires a separate admitted extension operation.
- Add bounded operator snapshots and readiness workflows over canonical observations and receipts rather than logs or exporter state.
- Add positive and negative conformance for cardinality, secret labels, exporter failure, corrupt storage, stale observations, partial scans, and evidence overclaims.

## Impact

- **Files**: fabric observation primitives, exporter and scan adapters, node/system-extension health integration, metrics and tracing shells, operator readback, readiness workflows, fixtures, and a new `fabric-observability` accepted spec.
- **Testing**: aggregation and health properties, exporter parity/failure, bounded cardinality, redaction, corruption and partial-scan detection, stale snapshot denial, read-only integrity, and repair-authority denial.
- **Safety**: metrics, traces, health, integrity results, and readiness receipts are observations; they do not grant authority, prove service correctness, authorize repair, or establish production readiness alone.
- **Licensing**: Aspen `main` observability and integrity behavior may guide requirements, but implementation reuse requires compatible provenance.
