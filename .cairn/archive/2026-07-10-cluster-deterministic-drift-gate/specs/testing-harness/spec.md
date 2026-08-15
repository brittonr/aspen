## ADDED Requirements

### Requirement: Cluster lifecycle drift gate reruns fresh roots
r[molten.testing.cluster_drift.lifecycle_rerun_gate] Molten MUST provide a deterministic drift gate that runs cluster lifecycle workflows in fresh isolated state roots with the same declared inputs and compares canonical evidence refs and normalized values through explicit allowed-variance declarations.

#### Scenario: Same cluster lifecycle rerun is stable
- GIVEN a declared two-node cluster lifecycle input and two fresh isolated state roots
- WHEN the drift gate runs `cluster init`, `cluster start`, `cluster status`, and `cluster stop` in both roots
- THEN semantic manifest, node receipt, control receipt, decision, and validation refs match after declared non-semantic variance is normalized
- AND the drift receipt binds both run summaries and the variance declarations used.

#### Scenario: Already-running path remains deterministic
- GIVEN equivalent already-running cluster state produced from declared inputs
- WHEN the drift gate reruns `cluster start`
- THEN the already-running evidence and status refs match semantically
- AND rendered output differences alone do not affect pass evidence.

### Requirement: Cluster drift negatives reject ambient and retry-only stability
r[molten.testing.cluster_drift.ambient_state_negatives] Molten MUST reject cluster drift pass claims when evidence changes due to undeclared ambient state, runtime paths, ordering, stale child refs, unstable map ordering, retry-only success, or rendered-output-only success.

#### Scenario: Undeclared child ref drift fails closed
- GIVEN two cluster evidence summaries with a changed canonical child receipt ref and no matching variance declaration
- WHEN the drift comparator evaluates the summaries
- THEN the decision is deny
- AND diagnostics name the first differing semantic ref.

#### Scenario: Retry does not mask cluster drift
- GIVEN a cluster workflow that alternates between two canonical evidence refs across fresh runs
- WHEN the drift gate is invoked
- THEN the gate reports drift instead of retrying until two matching outputs appear
- AND the retry result is not accepted as deterministic pass evidence.
