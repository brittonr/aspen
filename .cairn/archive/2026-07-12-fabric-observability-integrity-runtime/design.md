## Context

Molten's evidence model is richer than ordinary telemetry, while live metrics and integrity checks are still subsystem-specific or fixture-oriented. A shared fabric contract must preserve that distinction: telemetry is bounded operational observation, and integrity checks report detected relationships without granting repair authority or proving whole-system correctness.

## Decisions

### 1. Observation schemas and decisions are pure

**Choice:** Canonical metric descriptors, label sets, events, health inputs, readiness policies, integrity plans, scan observations, findings, and receipt payloads are validated and evaluated without I/O.

**Rationale:** Exporter availability, filesystem ordering, clocks, and process state must not change health or integrity semantics invisibly.

### 2. Collection and export are adapters

**Choice:** Tracing, Prometheus, OpenTelemetry, durable-state scanning, content verification, runtime counters, and simulation observations implement versioned adapter profiles. Extensions emit canonical observations rather than importing exporter APIs.

**Rationale:** Operational backends are replaceable effects, not core or extension semantics.

### 3. Cardinality, size, and confidentiality are bounded

**Choice:** Profiles cap descriptors, labels, label values, events, bytes, series, scan items, findings, and snapshot frequency. Secret, credential, private-path, raw-ticket, payload, and unbounded peer/item identifiers are rejected or redacted before export.

**Rationale:** Telemetry must not become a denial-of-service or exfiltration channel.

### 4. Integrity checking is read-only by default

**Choice:** Integrity plans inspect declared durable namespaces, logs, snapshots, content manifests, indexes, receipts, and checkpoints and produce findings or repair recommendations. Mutation requires a separate admitted repair, retention, quarantine, or recovery operation.

**Rationale:** Detecting inconsistency does not establish the correct repair or authorize destructive action.

### 5. Health and readiness name evidence scope

**Choice:** Health transitions consume bounded current observations with freshness and profile refs. Readiness workflows distinguish local component health, extension readiness, cluster evidence, and production/release claims; no lower class silently satisfies a stronger one.

**Rationale:** A green exporter or local scan is not distributed correctness or release readiness.

### 6. Exporter failure does not alter service semantics

**Choice:** Telemetry loss, backpressure, or exporter outage is classified and bounded. Policy may degrade or stop a service when observability is required, but adapters cannot block indefinitely or mutate extension state outside declared supervision decisions.

**Rationale:** Observability should not create hidden control flow.

## Functional core / imperative shell split

- Pure core: descriptor and label validation, aggregation, health/readiness transitions, integrity plans and predicates, finding classification, redaction decisions, claim-ladder checks, and receipt payloads.
- Shell: collect live observations, read declared stores, stream exporter data, enforce queues/timeouts, persist snapshots, and render bounded operator views.

## Dependencies

- System-extension runtime.
- Fabric durable-state, transport, time, resource, and simulation profiles.
- Existing evidence, retention, provenance, node health, and operator workflow boundaries.

## Risks / Trade-offs

- A universal metrics vocabulary can become rigid. Standardize only fabric-level dimensions and permit namespaced extension descriptors.
- Full scans can disrupt workloads. Require bounded incremental plans, resource admission, and explicit consistency caveats.
- Operators may overread health. Every view and receipt carries freshness, scope, unavailable states, and non-claims.
