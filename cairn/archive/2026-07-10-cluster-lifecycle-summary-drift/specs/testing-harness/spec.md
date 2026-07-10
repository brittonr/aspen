## ADDED Requirements

### Requirement: Cluster lifecycle summaries support deterministic drift comparison
r[molten.testing.cluster_lifecycle_summary_drift.receipt_summary] Molten MUST derive drift-ready cluster lifecycle summaries from canonical lifecycle receipt fields or equivalent in-memory observations, including workflow id, node ids, manifest refs, per-node lifecycle refs, command decisions, already-running observations, stop order, variance refs, and caveats.

#### Scenario: Equivalent lifecycle runs compare stable summaries
- GIVEN the same declared cluster lifecycle input and two fresh isolated state roots
- WHEN the harness runs both workflows and derives drift summaries
- THEN semantic refs and decisions match after explicitly declared temporary-root, runtime-path, store-path, diagnostic-log, or rendered-output variance is normalized
- AND the drift receipt binds the compared fields and variance declarations.

#### Scenario: Already-running path is included in the summary
- GIVEN equivalent already-running cluster states
- WHEN repeated `cluster start` observations are summarized
- THEN already-running status refs and decisions are compared as semantic fields
- AND startup artifacts are not rewritten to hide drift.

### Requirement: Cluster lifecycle drift negatives fail closed
r[molten.testing.cluster_lifecycle_summary_drift.negatives] Molten MUST deny cluster lifecycle drift pass evidence when child refs, node order, field kinds, required fields, ambient state, undeclared volatile values, retry-only success, or rendered-output-only success diverge across reruns.

#### Scenario: Node ordering drift denies
- GIVEN two lifecycle summaries with the same node ids but different semantic order
- WHEN the drift comparator evaluates them
- THEN the decision is deny
- AND diagnostics name the node-order field as the first divergence.

#### Scenario: Retry-only stability is not accepted
- GIVEN a lifecycle workflow that only becomes stable after retrying failed or drifted attempts
- WHEN deterministic drift evidence is requested
- THEN retry success remains diagnostic-only
- AND the drift gate does not accept it as deterministic pass evidence.
