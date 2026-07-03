# Tasks: job-dag-scheduler-proof

## Phase 1: Topology proof

- [ ] [serial] r[molten.job_dag_state_machine_proof.topological_order_determinism] Add generated acyclic DAG tests proving deterministic order ids, node index maps, and dependency indices.
- [ ] [parallel] r[molten.job_dag_state_machine_proof.topological_order_determinism] Add negative tests for duplicate node ids, unknown edge endpoints, cycles, and out-of-range dependency indices.

## Phase 2: Scheduler readiness

- [ ] [serial] r[molten.job_dag_state_machine_proof.dependency_readiness_gate] Add tests proving nodes run only when all dependency indices are complete.
- [ ] [parallel] r[molten.job_dag_state_machine_proof.dependency_readiness_gate] Add negative tests for unsatisfied dependency attempts, missing executable refs, missing output slots, and stale completed-index refs.

## Phase 3: Worker schedule receipts

- [ ] [serial] r[molten.job_dag_state_machine_proof.worker_schedule_replay] Add tests proving worker schedule receipts bind request identity, stage order, completed indices, output refs, and replay identity.
- [ ] [parallel] r[molten.job_dag_state_machine_proof.worker_schedule_replay] Add negative tests for reordered schedules, mismatched output refs, and stale worker request refs.

## Phase 4: Validation

- [ ] [serial] r[molten.job_dag_state_machine_proof.topological_order_determinism] r[molten.job_dag_state_machine_proof.dependency_readiness_gate] r[molten.job_dag_state_machine_proof.worker_schedule_replay] Add traceability evidence and run `cargo test job`.
