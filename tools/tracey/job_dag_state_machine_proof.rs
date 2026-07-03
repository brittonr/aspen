//! Tracey markers for job DAG scheduler state-machine proof coverage.
//!
//! Implementation and verification live in `src/job/**`; Aspen's current Cairn
//! Tracey scanner records references from `tools/**`, so this file anchors the
//! accepted proof IDs to focused job DAG tests.
//!
//! Verification evidence:
//! - `src/job/parts/dag/tests/m000/p007/body.rs`

// r[verify molten.job_dag_state_machine_proof.topological_order_determinism]
// r[verify molten.job_dag_state_machine_proof.dependency_readiness_gate]
// r[verify molten.job_dag_state_machine_proof.worker_schedule_replay]
