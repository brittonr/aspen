## ADDED Requirements

### Requirement: Cluster harness CLI emits receipt-first lifecycle evidence
r[molten.testing.receipt_first_cluster_harness.cli_receipt_surface] Molten MUST provide a receipt-first cluster harness command surface for evidence-bearing cluster lifecycle workflows. The command surface MUST bind a fixture-derived or explicitly supplied plan, manifest ref, ordered node ids, phase decisions, per-node child receipt refs, already-running observations, reverse stop order, diagnostic refs, allowed variance refs, and evidence-only caveats into a canonical `cluster-lifecycle-run-v1` receipt before pass evidence is accepted.

#### Scenario: Fixture-backed lifecycle writes a parent receipt
- GIVEN a checked cluster scenario fixture and an explicit cluster state root
- WHEN the receipt-first cluster harness executes `init`, `start`, `status`, and `stop` phases
- THEN it writes a canonical cluster lifecycle receipt that binds the manifest, phase decisions, node order, child receipt refs, stop order, diagnostics, variance refs, and caveats
- AND rendered stdout is recorded only as a diagnostic view.

#### Scenario: Stdout-only lifecycle success is rejected
- GIVEN a cluster command sequence whose terminal output says success but no canonical lifecycle receipt is written
- WHEN pass evidence is evaluated
- THEN the cluster harness denies the pass claim
- AND diagnostics state that stdout does not replace lifecycle receipt evidence.

### Requirement: Cluster run artifact directories are verifiable offline
r[molten.testing.receipt_first_cluster_harness.run_artifact_directory] Molten MUST write and verify cluster run artifact directories that bind fixture metadata, derived plans, lifecycle receipts, per-node child receipt refs, reconciliation refs, drift summaries, diagnostic-log refs, failure-bundle refs when present, and evidence-only caveats. Offline verification MUST recompute canonical refs and deny when artifact kinds, required child refs, variance declarations, or caveats diverge from the fixture metadata.

#### Scenario: Offline verifier accepts a complete run directory
- GIVEN a cluster run directory containing fixture metadata, derived plan evidence, lifecycle receipt, per-node child receipts, reconciliation receipt, drift summary, diagnostic-log refs, and caveats
- WHEN offline verification recomputes refs and compares the run against the fixture metadata
- THEN verification passes and emits a canonical verification receipt
- AND the receipt preserves the run's local, VM, unavailable, or diagnostic-only evidence scope.

#### Scenario: Tampered artifact directory denies
- GIVEN a cluster run directory whose child receipt, artifact kind, drift summary, variance declaration, or caveat was modified after the lifecycle receipt was written
- WHEN offline verification evaluates the directory
- THEN verification denies before pass evidence is accepted
- AND diagnostics identify the mismatched field or missing ref.

### Requirement: Fixture-derived plans drive executable cluster tiers
r[molten.testing.receipt_first_cluster_harness.fixture_executable_runner] Molten MUST use checked multinode scenario fixture metadata as the source of truth for executable cluster tiers, including local multiprocess and VM-backed runs. The executable shell MUST spawn child processes only after pure plan validation passes, and the resulting receipt MUST bind isolated state-root handles, transport handles, command-plan refs, expected receipt refs, timeout policy, cleanup policy, observed child refs, cleanup observations, and local-evidence caveats.

#### Scenario: Local multiprocess runner follows fixture metadata
- GIVEN a checked multinode scenario fixture with a local multiprocess execution profile, isolated node handles, transport handles, expected artifact kinds, and required receipt refs
- WHEN the executable runner launches child node processes
- THEN the run receipt binds the fixture-derived plan, startup refs, workflow refs, shutdown refs, cleanup refs, timeout observations, and caveats
- AND the receipt states that local multiprocess evidence is not VM, WAN, deployment, or production-readiness evidence.

#### Scenario: Invalid fixture plan denies before spawn
- GIVEN a fixture-derived local multiprocess plan with colliding state-root handles, colliding transport handles, stale tickets, unsupported pass claims, or missing expected receipts
- WHEN the executable runner prepares the run
- THEN it denies before spawning child processes
- AND diagnostics identify the invalid plan field.

### Requirement: Cluster harness failures produce first-divergence diagnostics
r[molten.testing.receipt_first_cluster_harness.failure_triage] Molten MUST emit first-divergence diagnostics for denied cluster harness runs and MAY export sealed diagnostic failure bundles. The diagnostics MUST identify the first missing, stale, mismatched, unsupported, or undeclared semantic field from the fixture/run comparison, and sealed failure bundles MUST remain diagnostic-only evidence.

#### Scenario: Missing child receipt reports first divergence
- GIVEN a fixture-backed cluster run whose lifecycle receipt lacks a required startup, health, control, shutdown, queue, dispatch, reconcile, ack, or protocol child ref
- WHEN failure triage evaluates the denied run
- THEN the first-divergence diagnostic names the missing field, expected artifact kind, and affected node or phase
- AND the failure bundle, if exported, remains diagnostic-only.

#### Scenario: Diagnostic failure bundle cannot pass a gate
- GIVEN a verified sealed cluster failure bundle with first-divergence diagnostics
- WHEN a pass evidence gate evaluates the bundle
- THEN the gate denies pass evidence
- AND diagnostics state that failure triage does not grant authority, policy, provenance, resource, transport, VM, deployment, production-readiness, or release trust.
