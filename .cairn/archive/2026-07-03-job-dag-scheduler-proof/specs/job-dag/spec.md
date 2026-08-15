## ADDED Requirements

### Requirement: Job DAG topological order is deterministic
r[molten.job_dag_state_machine_proof.topological_order_determinism] Molten MUST prove that valid acyclic job DAGs produce deterministic topological order ids, node index maps, and dependency indices, while duplicate nodes, unknown edge endpoints, and cycles deny before execution.

#### Scenario: Generated acyclic DAG has stable order
- GIVEN a generated bounded acyclic job DAG
- WHEN Molten computes the execution plan more than once
- THEN the order ids, node indices, and dependency indices match
- AND every edge points from an earlier or completed dependency to a later dependent node.

### Requirement: Job scheduler admits only dependency-ready nodes
r[molten.job_dag_state_machine_proof.dependency_readiness_gate] Molten MUST prove that worker scheduling admits a job node only when all dependency indices for that node have completed, and unsatisfied dependency attempts MUST deny before execution.

#### Scenario: Unsatisfied dependency denies worker run
- GIVEN a job node whose dependency index is not present in the completed set
- WHEN the worker scheduler attempts to run the node
- THEN admission denies
- AND no stage execution receipt is emitted for that node.

### Requirement: Job worker schedule receipts replay deterministically
r[molten.job_dag_state_machine_proof.worker_schedule_replay] Molten MUST prove worker schedule receipts bind request identity, stage order, completed indices, output refs, diagnostics, and replay identity so reordered, stale, or mismatched schedules fail closed.

#### Scenario: Reordered worker schedule denies replay
- GIVEN a recorded worker schedule receipt and a schedule replay with stages reordered
- WHEN Molten validates replay identity
- THEN validation denies
- AND diagnostics identify the stage order or output-ref mismatch.
