# Fabric observability and integrity

Molten separates operational observations from evidence, authority, and service semantics. Pure bounded decisions live in `crates/molten-core/src/fabric_observability/`; canonical Preserves projection, exporter rendering, capability-rooted reads, tracing effects, and operator composition live in `src/fabric_observability/`.

## Canonical observations

The fabric defines profile, descriptor, sample, event, health input, readiness policy, health decision, integrity plan, scan observation, finding, adapter profile/status/outcome, and snapshot records. Every observation names its source, profile, claim scope, generation, observation and expiry ticks, resource ref, evidence refs, and non-claims. Exporter formatting is not canonical identity.

Metric aggregation uses integer values and deterministic series identities. Descriptors select sum, last, minimum, or maximum semantics; counters require sum. Labels are validated against a finite descriptor vocabulary, sorted canonically, and bounded by profile. Duplicate samples, descriptors, labels, and series fail closed.

## Confidentiality and cardinality

Labels classify their values. Credentials, secrets, private paths, raw tickets, payloads, and unbounded identifiers require a reviewed field-specific redaction rule; otherwise validation denies export. Public values with known secret, token, private-key, ticket, or absolute-path markers are rejected as misclassified. This is a defense-in-depth check, not a claim that arbitrary secret text can always be recognized.

Profiles bound descriptors, labels and label bytes, series, events and detail bytes, queue bytes, snapshots and frequency, scan items, findings, and diagnostics. Exceeding a bound produces a typed denial, backpressure, drop, partial, unavailable, or failure outcome rather than silent truncation or unbounded growth.

## Export adapters

Versioned fabric adapter classes cover tracing, Prometheus, OpenTelemetry, durable-state scans, content verification, runtime counters, and deterministic simulation. Extensions provide canonical observations; they do not receive exporter registries, tracing subscribers, files, Redb handles, Iroh handles, or simulator runtimes.

- The Prometheus shell emits bounded text exposition from validated public series.
- The OpenTelemetry shell emits a deterministic newline-delimited JSON envelope suitable for an admitted sink.
- The tracing shell emits the canonical observation ref, not raw event payload or labels.
- Deterministic simulation uses the same renderer and delivery decision core with a bounded recording sink.

The generic sink boundary reports completion, drops, and typed failure. Availability, queue pressure, timeout, frequency, cancellation, permission denial, unsupported capability, corrupt input, and adapter failure are terminal canonical outcomes. There is no hidden retry. This implementation does not claim a live OTLP network collector or Prometheus server; a deployment sink remains an explicit adapter effect.

## Read-only integrity

Integrity plans enumerate canonical target refs and finite item/finding/byte limits. The durable-state adapter reads only through `NodeStateNamespace` capability roots. It never serializes host paths into observations. The content adapter accepts a read-only bounded source interface with no mutation methods. Both produce the same scan observations and pure result evaluation.

Missing, corrupt, mismatched, unexpected, over-bound, permission-denied, unsupported, cancelled, unavailable, and partial observations become bounded findings. A complete pass requires the declared inventory to be exhausted. Findings carry recommendations only and always set mutation authority to false. Repair, quarantine, retention, recovery, or deletion requires a separate operation-specific authority and policy ref that explicitly targets the finding.

## Health, readiness, and operator views

Node and system-extension status project into the canonical health model. Readiness consumes supplied ticks rather than reading a clock, rejects stale or missing required sources, and preserves local-component, adapter, system-extension, cluster, operator, and release-production scopes. Promotion beyond the strongest supplied scope requires explicit scope evidence. A local healthy node cannot silently become cluster or release readiness.

Operator snapshots are bounded canonical artifacts over current series and evidence refs. They can report pass, degraded, unavailable, or deny for the declared scope. They do not grant capabilities, authorize repair, prove service correctness, establish global cluster truth, or establish release eligibility.
