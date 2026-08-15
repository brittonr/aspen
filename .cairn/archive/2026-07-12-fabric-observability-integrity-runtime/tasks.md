## Phase 1: Observation and integrity primitives

- [x] [serial] Define canonical metric, event, health, readiness, integrity-plan, scan, finding, adapter-status, and snapshot models with scope, freshness, bounds, redaction, evidence, and non-claims. r[molten.fabric_observability.model]
- [x] [serial] Implement pure descriptor/label validation, aggregation, health/readiness transitions, integrity predicates, finding classification, redaction, and claim-scope decisions. r[molten.fabric_observability.pure_core] r[molten.fabric_observability.health_scope]
- [x] [parallel] Add positive and negative primitive tests for stable aggregation, malformed descriptors, stale observations, unavailable inputs, health transitions, finding scope, redaction, and claim promotion. r[molten.fabric_observability.pure_core] r[molten.fabric_observability.health_scope]

## Phase 2: Collection and export adapters

- [x] [serial] Define versioned tracing, metric export, durable-state scan, content verification, runtime counter, and simulation observation adapter contracts. r[molten.fabric_observability.adapter_contract]
- [x] [parallel] Add bounded tracing, Prometheus, and OpenTelemetry shells that consume canonical observations without exposing backend handles to extensions. r[molten.fabric_observability.adapter_contract] r[molten.fabric_observability.bounds_redaction]
- [x] [parallel] Add bounded durable-state and content integrity scan adapters that produce read-only observations and findings through admitted capability roots and ports. r[molten.fabric_observability.integrity_readonly]
- [x] [parallel] Implement explicit exporter unavailable, backpressure, timeout, dropped observation, partial scan, stale observation, cancellation, and adapter failure outcomes. r[molten.fabric_observability.failure_semantics]

## Phase 3: System-extension and operator integration

- [x] [serial] Route node and system-extension health through canonical current observations and preserve local, adapter, extension, cluster, operator, and release claim scopes. r[molten.fabric_observability.health_scope]
- [x] [parallel] Enforce descriptor, label, series, event, queue, byte, scan, finding, frequency, and diagnostic limits plus secret/path/ticket/payload redaction. r[molten.fabric_observability.bounds_redaction]
- [x] [parallel] Add bounded operator snapshots and readiness workflows over canonical observations and receipts, with unavailable states and explicit non-claims. r[molten.fabric_observability.health_scope]
- [x] [parallel] Require separate admitted repair, quarantine, retention, recovery, or deletion operations before any integrity finding can cause mutation. r[molten.fabric_observability.integrity_readonly]

## Phase 4: Conformance and validation

- [x] [serial] Run shared positive and negative adapter conformance for aggregation, exporter rendering/failure, live/simulation parity, cardinality, redaction, corruption, partial scans, read-only integrity, cleanup, and telemetry-as-authority denial. r[molten.fabric_observability.final_validation]
- [x] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and the smallest relevant Nix checks before sync and archive. r[molten.fabric_observability.final_validation]
