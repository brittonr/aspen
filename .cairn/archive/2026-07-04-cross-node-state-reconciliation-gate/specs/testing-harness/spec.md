## ADDED Requirements

### Requirement: Cross-node reconciliation gate binds distributed state refs
r[molten.testing.multinode.cross_node_reconciliation_gate] Molten SHOULD provide a cross-node reconciliation gate that compares explicit per-node evidence summaries against declared topology, scenario fixture, required receipt refs, expected equality classes, and allowed variance refs.

#### Scenario: Converged nodes pass reconciliation
- GIVEN a multinode run with per-node evidence summaries, matching topology refs, required workflow receipts, and declared equality classes
- WHEN the reconciliation gate evaluates the run
- THEN the gate emits a pass receipt binding node summaries, compared refs, allowed variance refs, diagnostics, and evidence-only caveats
- AND matching refs prove only the declared reconciliation scope.

#### Scenario: Declared variance is visible
- GIVEN a multinode run where selected per-node refs are allowed to differ
- WHEN the reconciliation gate evaluates those refs
- THEN the pass receipt binds the variance declaration that permits the difference
- AND undeclared differences remain denial conditions.

### Requirement: Reconciliation denies stale, missing, divergent, or log-only evidence
r[molten.testing.multinode.reconciliation_deny_drift] Molten MUST reject reconciliation pass claims when required node evidence is missing, stale, wrong-topology, duplicated, divergent without variance, or represented only by logs.

#### Scenario: Divergent queue ref denies
- GIVEN two node summaries that should share an expected queue or dispatch outcome but report different refs without declared variance
- WHEN the reconciliation gate evaluates the summaries
- THEN the gate denies before emitting pass evidence
- AND diagnostics identify the divergent ref class.

#### Scenario: Duplicate semantic commit denies
- GIVEN a multinode run where duplicate delivery produced more than one semantic commit for the same operation id
- WHEN reconciliation evaluates committed operation evidence
- THEN the gate denies the pass claim and identifies duplicate commit drift.
