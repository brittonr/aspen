## ADDED Requirements

### Requirement: Cluster CLI lifecycle is receipt-backed
r[molten.testing.cluster_cli.lifecycle_roundtrip] Molten MUST test the `cluster init`, `cluster start`, `cluster status`, and `cluster stop` CLI lifecycle over at least two isolated node roots, and the test MUST assert canonical manifest and node receipt evidence rather than treating rendered output as authoritative pass evidence.

#### Scenario: Two-node cluster lifecycle records receipts
- GIVEN an explicit cluster state root and two safe node names
- WHEN the CLI harness runs `cluster init`, `cluster start`, `cluster status`, and `cluster stop`
- THEN the cluster manifest binds the expected node ids and each node root contains the required config, identity, startup, health or control, heartbeat, shutdown, and control receipt artifacts
- AND stdout and stderr remain diagnostic-only views over the canonical receipts.

#### Scenario: Already-running start does not mutate unrelated evidence
- GIVEN a cluster whose nodes are already running
- WHEN `cluster start` is invoked again
- THEN the command reports already-running status from canonical node status evidence
- AND it does not replace unrelated node lifecycle artifacts.

### Requirement: Cluster CLI negative fixtures fail closed
r[molten.testing.cluster_cli.fail_closed_negatives] Molten MUST reject cluster CLI pass evidence when the manifest is missing, malformed, empty, stale, or inconsistent with node lifecycle state; unsafe or duplicate node names MUST fail before node-root mutation unless `--force` explicitly resets only planned node roots.

#### Scenario: Malformed manifest denies lifecycle commands
- GIVEN a cluster state root with a missing, empty, malformed, or unsupported `cluster.nodes` manifest
- WHEN `cluster start`, `cluster status`, or `cluster stop` reads the cluster plan
- THEN the command fails closed before operating on node roots
- AND diagnostics identify the manifest problem.

#### Scenario: Reinitialization requires explicit destructive intent
- GIVEN a planned node root already has initialized lifecycle state
- WHEN `cluster init` runs without `--force`
- THEN the command denies before removing or overwriting that node root
- AND a forced run removes only the planned node roots and writes fresh lifecycle evidence.
