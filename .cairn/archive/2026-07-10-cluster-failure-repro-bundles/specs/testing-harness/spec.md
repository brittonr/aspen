## ADDED Requirements

### Requirement: Cluster failures export sealed diagnostic repro bundles
r[molten.testing.cluster_failure_repro_bundles.bundle_schema] Molten MUST export sealed diagnostic repro bundles for denied, unavailable, or failed-validation cluster-related runs, and each bundle MUST bind scenario fixture refs, topology refs, command or scheduler refs, seed or effect-log refs, node summary refs, child receipt refs, diagnostic refs, diagnostic-log refs, redaction policy refs, replay status, private attachment refs, reveal receipt refs, and evidence-only caveats.

#### Scenario: Cluster lifecycle denial exports a sealed bundle
- GIVEN a cluster lifecycle run that denies because required canonical evidence is missing or stale
- WHEN failure repro export is requested
- THEN the bundle payload binds the scenario or command refs, node summaries, child receipts, diagnostics, logs, redaction policy, replay status, and caveats
- AND the bundle seal ref matches the payload ref.

#### Scenario: VM unavailable evidence exports non-replayable diagnostics
- GIVEN a VM run whose host support is unavailable or whose fault validation denies
- WHEN failure repro export is requested
- THEN the bundle records non-replayable VM observation status
- AND it remains diagnostic-only evidence.

### Requirement: Cluster failure bundles preserve privacy and cannot satisfy pass gates
r[molten.testing.cluster_failure_repro_bundles.privacy_and_nonpass] Molten MUST reject cluster failure bundle verification, unpacking, or pass-gate use when bundle payload refs are tampered, redaction evidence is missing or stale, private attachments lack exact reveal receipts, replay status is incompatible with deterministic pass evidence, or the bundle is diagnostic-only.

#### Scenario: Tampered bundle fails verification
- GIVEN a sealed cluster failure repro bundle
- WHEN its payload, seal metadata, child receipt refs, or redaction refs are modified
- THEN verification denies before materializing private content or pass evidence
- AND diagnostics name the stale or tampered field.

#### Scenario: Diagnostic failure bundle cannot pass a gate
- GIVEN a verified diagnostic cluster failure bundle
- WHEN a pass evidence gate evaluates it
- THEN the gate denies pass evidence
- AND diagnostics state that failure repro bundles are diagnostic-only.
