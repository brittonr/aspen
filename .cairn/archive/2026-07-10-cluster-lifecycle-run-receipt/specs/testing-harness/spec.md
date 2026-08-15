## ADDED Requirements

### Requirement: Cluster lifecycle emits a canonical run receipt
r[molten.testing.cluster_lifecycle_receipt.run_receipt] Molten MUST emit or produce a canonical `cluster-lifecycle-run-v1` receipt for evidence-bearing cluster lifecycle workflows, and the receipt MUST bind the manifest ref, ordered node ids, command phase decisions, per-node lifecycle receipt refs, already-running observations, stop ordering, diagnostics, and evidence-only caveats.

#### Scenario: Two-node lifecycle produces one reviewable receipt
- GIVEN an explicit cluster state root, two safe node names, and a complete `init -> start -> status -> stop` workflow
- WHEN the lifecycle harness finalizes cluster evidence
- THEN it emits a `cluster-lifecycle-run-v1` receipt whose manifest, node order, per-node lifecycle refs, status refs, shutdown refs, and stop order match the observed canonical artifacts
- AND rendered stdout or stderr remain diagnostic-only views over the receipt.

#### Scenario: Already-running start is bound without rewriting startup evidence
- GIVEN a cluster whose nodes already have valid startup and health evidence
- WHEN the lifecycle receipt records a repeated `cluster start`
- THEN the receipt binds already-running status refs
- AND it shows that unrelated startup evidence was not replaced.

### Requirement: Cluster lifecycle receipts fail closed on missing or stale evidence
r[molten.testing.cluster_lifecycle_receipt.fail_closed_validation] Molten MUST deny cluster lifecycle pass evidence when required phase receipts are missing, node summaries are duplicated, node order diverges from the manifest, manifest refs are stale, canonical parsing fails, or rendered output is the only evidence of success.

#### Scenario: Missing phase receipt denies pass evidence
- GIVEN a cluster lifecycle summary with an omitted startup, health, shutdown, or control receipt required by the declared phase
- WHEN the receipt validator evaluates the summary
- THEN it emits a deny decision
- AND diagnostics name the missing phase and node before pass evidence can be used.

#### Scenario: Rendered-output-only success is rejected
- GIVEN a cluster command whose stdout says success but whose canonical receipt artifact is missing or stale
- WHEN the lifecycle receipt is built
- THEN the receipt denies pass evidence
- AND the diagnostic states that logs or rendered output are not authoritative evidence.
