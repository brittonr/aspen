# Bounded observability and health

Molten's operational observations are bounded, scoped inputs to decisions, not a secondary authority channel. This article assumes familiarity with observation profiles, content refs, and the [fabric observability contract](../../fabric-observability.md). It explains aggregation and readiness mechanics, then shows how an apparently healthy component can correctly fail a broader readiness review. The [Technical companion](../README.md) links adjacent operational topics.

## Three layers that should remain separate

A useful reading separates deterministic evaluation, effectful collection or export, and operational interpretation. The pure core receives profiles, descriptors, samples, health inputs, and supplied ticks. It does not need an ambient clock or an exporter connection to decide whether those values satisfy its rules. Shell adapters collect observations, render output, access admitted sources, and deliver to sinks. Operators interpret canonical outcomes within their declared scope.

According to the [governing document](../../fabric-observability.md), canonical observations identify their source, profile, generation, observation and expiry ticks, resource ref, evidence refs, claim scope, and non-claims. Prometheus text or an OpenTelemetry-oriented JSON envelope is a projection, not a replacement identity. A successful exporter write does not independently establish a healthy service, and a healthy service observation does not imply that export completed.

## Bounded aggregation is a partial computation

`aggregate_metric_samples` in the [aggregation implementation](../../../crates/molten-core/src/fabric_observability/aggregation.rs) validates the profile and descriptor collection, indexes descriptors, and tracks sample refs before accumulation. Duplicate sample refs are reported rather than counted twice. Missing descriptors and sample-validation failures add issues. If any issues remain, the function returns an error rather than a successful partial collection.

Series identity is the descriptor ref plus labels. A new identity is rejected when the profile's series bound has already been reached. This distinction matters: a label vocabulary may be finite while its combinations still exceed the admitted series count. Reviewing cardinality therefore means reviewing both the vocabulary and the resulting combinations, not merely counting metric names.

Values are integers. Sum uses checked addition and reports `ArithmeticOverflow`; minimum and maximum select extrema. Last-value aggregation orders candidates by `(observed_tick, sample_ref)`, making simultaneous-tick selection explicit rather than dependent on arrival order. The accumulator retains sorted, deduplicated source refs. These properties support reproducible reasoning over admitted values; they do not establish that a sensor accurately measured an external event.

Confidentiality is another admission dimension. The governing contract requires field-specific reviewed redaction for sensitive classes and rejects recognizable secret or absolute-path markers falsely labeled public. This is defense in depth, not universal secret recognition. A low-cardinality label can still disclose a credential; a redacted label can still produce too many series. Neither review substitutes for the other.

## Readiness combines state, freshness, and scope

The [health evaluator](../../../crates/molten-core/src/fabric_observability/health.rs) indexes inputs by source ID and visits the policy's required sources. Missing sources make state unavailable. For a present source, `as_of_tick > valid_until_tick` produces a stale observation and unavailable state. Equality at the expiry tick is therefore not stale under this particular comparison. Other input-validation issues can still deny readiness.

The evaluator combines state using severity, gathers supporting refs, and tracks the strongest supplied scope. Requesting a stronger target scope without scope evidence produces `ClaimScopeOverreach`. It then decides readiness separately from the state value. Structural and scope issues deny; missing, stale, or unavailable required observations can produce unavailable. Degraded input produces a degraded decision only when the policy permits it. A result's state and readiness should be read together rather than collapsed into a single green/red field.

Notably, `observation_authority_decision()` returns deny. This makes the conceptual boundary concrete: health evidence is not a capability to repair, delete, quarantine, or restart a resource.

## Worked example: freshness is not aggregation success

Consider an illustrative policy requiring storage and transport observations at tick 101. Storage is healthy and valid through tick 120. Transport is healthy in its recorded state but valid only through tick 100. All descriptors and metric samples are valid, and their bounded aggregation succeeds.

Readiness nevertheless becomes unavailable because the required transport health observation is stale. Lowering a displayed queue depth cannot fix that missing temporal evidence. Replacing the transport observation with one valid through tick 101 removes this particular staleness issue, but not every possible readiness obstacle.

Suppose both observations are local-component scope while the target is cluster scope and scope evidence is absent. The evaluator also reports scope overreach and denies readiness. Freshness and scope are independent checks: refreshing a local observation does not enlarge what it can support. Supplying well-formed scope refs satisfies a structural part of this model; this article does not claim that reference presence proves the truth of external evidence.

## Operator review and explicit limits

The [production runbook's observability snapshot](../../production-operator-runbooks.md) binds adapter health, queue pressure, control-loop evidence, resource pressure, retention drift, source-gate freshness, transport evidence, and import/export failure evidence. Its degraded outcome prevents over-limit pressure from being presented as pass evidence.

Suggested verification is to review aggregation boundaries, duplicate sample rejection, integer overflow, the exact expiry comparison, missing sources, degraded-policy handling, and scope overreach. Independently exercise the selected sink's unavailable, permission-denied, timeout, and bounded-queue outcomes. The documented generic sink has no hidden retry, and the implementation does not claim a live OTLP collector or Prometheus server merely because it can render their output shapes.

No such runtime checks were executed for this article. Read-only integrity findings remain recommendations without mutation authority; a complete scan requires exhausting its declared inventory. Neither an observation snapshot nor a health decision proves global cluster truth, service correctness, production performance, or release eligibility.

## Sources

- [Fabric observability and integrity](../../fabric-observability.md)
- [Production operator runbooks](../../production-operator-runbooks.md)
- [Metric aggregation implementation](../../../crates/molten-core/src/fabric_observability/aggregation.rs)
- [Health and readiness implementation](../../../crates/molten-core/src/fabric_observability/health.rs)
- [Technical companion](../README.md)
