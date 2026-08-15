# Fabric Observability Specification

## Purpose

Defines the `fabric-observability` capability.

## Requirements

### Requirement: Fabric observations are canonical and bounded
r[molten.fabric_observability.model] Molten MUST define canonical metric descriptor, sample, event, health input, health state, readiness policy, integrity plan, scan observation, finding, adapter status, and snapshot records with explicit schema, source, scope, generation, freshness, resource, redaction, evidence, and non-claim fields.

#### Scenario: Valid observation is canonical
- GIVEN a service emits a supported metric descriptor and bounded label set
- WHEN observation validation runs
- THEN it MUST produce stable canonical identity independent of exporter formatting.

#### Scenario: Unknown behavior-affecting field is rejected
- GIVEN an observation contains an unsupported field that changes scope, authority, or interpretation
- WHEN validation runs
- THEN it MUST deny rather than discard the field silently.

### Requirement: Aggregation and health decisions are pure
r[molten.fabric_observability.pure_core] Molten MUST evaluate descriptor compatibility, label policy, deterministic aggregation, health transitions, readiness decisions, integrity predicates, finding classes, redaction, and claim scope through pure functions over in-memory facts. Pure decisions MUST NOT read exporters, filesystems, clocks, processes, networks, or environment state.

#### Scenario: Same health facts produce the same state
- GIVEN identical current observations, freshness facts, policy, and prior health state
- WHEN health evaluation repeats
- THEN resulting state, diagnostics, and receipt payload MUST match.

### Requirement: Collection and export use adapters
r[molten.fabric_observability.adapter_contract] Molten MUST expose versioned adapter contracts for tracing, metric export, durable-state scanning, content verification, runtime counters, and deterministic simulation observations. Extensions MUST emit canonical observations and MUST NOT import Prometheus, OpenTelemetry, tracing backend, Redb, filesystem, Iroh, or simulator runtime objects.

#### Scenario: Prometheus adapter exports a canonical sample
- GIVEN a valid visible metric sample and admitted exporter binding
- WHEN export runs
- THEN the adapter MAY render backend-specific output while preserving the canonical descriptor and sample refs.

#### Scenario: Extension imports exporter internals
- GIVEN extension code requests a backend registry or exporter handle directly
- WHEN port validation or structural conformance runs
- THEN activation MUST deny the adapter bypass.

### Requirement: Telemetry cardinality and confidentiality are enforced
r[molten.fabric_observability.bounds_redaction] Molten MUST enforce admitted limits for descriptors, labels, label values, series, events, queued bytes, snapshots, scan items, findings, export frequency, and diagnostics. Secret bytes, credentials, raw tickets, private paths, payloads, and unbounded peer or item identifiers MUST be denied or transformed through approved redaction before export.

#### Scenario: Bounded labels export
- GIVEN metric labels use admitted names, values, visibility, and cardinality
- WHEN export validation runs
- THEN the sample MAY proceed within its resource envelope.

#### Scenario: Secret label is rejected
- GIVEN a label contains secret material or an unapproved raw path
- WHEN validation runs
- THEN export MUST deny or emit only an approved redacted marker.

### Requirement: Integrity checking is read-only by default
r[molten.fabric_observability.integrity_readonly] Molten MUST execute integrity plans as bounded read-only observations over declared durable namespaces, logs, snapshots, content manifests, indexes, receipts, and checkpoints unless a separate admitted repair, quarantine, retention, recovery, or deletion operation is supplied. Findings and recommendations MUST NOT grant mutation authority.

#### Scenario: Consistent store passes its checks
- GIVEN a bounded store inventory whose declared refs, lengths, indexes, and checkpoints agree
- WHEN an integrity plan evaluates it
- THEN the result MAY pass for that exact scope and observation freshness.

#### Scenario: Corruption does not trigger ambient repair
- GIVEN an integrity scan detects a corrupt chunk or log gap
- WHEN no repair authority and plan are present
- THEN it MUST emit a finding and leave state unchanged.

### Requirement: Health and readiness preserve evidence scope
r[molten.fabric_observability.health_scope] Molten MUST distinguish local component health, adapter health, system-extension readiness, cluster evidence, operator readiness, and release or production claims. Health and readiness transitions MUST bind freshness, unavailable observations, profile, environment, and supporting evidence; a lower scope MUST NOT silently satisfy a stronger claim.

#### Scenario: Fresh extension observations support readiness
- GIVEN all required extension and adapter observations are current and pass their declared checks
- WHEN extension readiness evaluates
- THEN it MAY pass for that extension and profile.

#### Scenario: Local health cannot prove cluster readiness
- GIVEN one node reports healthy local adapters without current peer or cluster evidence
- WHEN cluster readiness evaluates
- THEN it MUST deny or report unavailable rather than promote local health.

### Requirement: Exporter and scan failures are explicit and bounded
r[molten.fabric_observability.failure_semantics] Molten MUST classify exporter unavailable, backpressure, timeout, dropped observation, partial scan, permission denial, unsupported capability, stale observation, corrupt input, cancellation, and adapter failure explicitly. Adapters MUST NOT block indefinitely or mutate extension semantic state outside declared supervision policy.

#### Scenario: Exporter outage is observable
- GIVEN an exporter becomes unavailable
- WHEN its bounded queue or timeout policy is reached
- THEN adapter status MUST report the failure and apply the declared drop, degrade, or service-policy signal.

#### Scenario: Partial scan cannot pass as complete
- GIVEN an integrity scan stops before its declared inventory bound is completed
- WHEN result validation runs
- THEN it MUST report partial or unavailable and cannot satisfy a complete-integrity gate.

### Requirement: Observability and integrity have shared conformance
r[molten.fabric_observability.final_validation] Molten MUST include positive and negative conformance for canonical aggregation, health transitions, exporter rendering, live/simulation parity, bounded cardinality, redaction, exporter failure, stale observations, corruption, partial scans, read-only integrity, repair-authority denial, cleanup, and claim-scope enforcement.

#### Scenario: Conforming adapter passes
- GIVEN an adapter preserves canonical inputs, bounds, redaction, terminal events, and declared failures
- WHEN shared conformance runs
- THEN it MAY be admitted for its declared environment and evidence scope.

#### Scenario: Telemetry is not authority
- GIVEN a valid health, metrics, or integrity receipt is presented as capability or repair authority
- WHEN downstream admission evaluates it
- THEN admission MUST deny the overclaim.
