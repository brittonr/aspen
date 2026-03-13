//! Saga pattern implementation for distributed transactions.
//!
//! This module provides a saga/compensation pattern for managing distributed
//! transactions where multiple steps may need to be rolled back if any step fails.
//! The pattern follows LIFO (last-in-first-out) compensation order.
//!
//! # Example
//!
//! ```ignore
//! use aspen_jobs::{SagaBuilder, SagaExecutor};
//!
//! let saga = SagaBuilder::new("order_saga")
//!     .step("reserve_inventory")
//!         .done()
//!     .step("charge_payment")
//!         .done()
//!     .step("ship_order")
//!         .done()
//!     .build();
//!
//! let executor = SagaExecutor::new(store);
//! let state = executor.start_saga(saga, None).await?;
//! ```

mod builder;
mod executor;
mod types;

pub use builder::*;
pub use executor::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_saga_builder() {
        let saga = SagaBuilder::new("test_saga")
            .description("A test saga")
            .timeout(Duration::from_secs(60))
            .metadata("version", "1.0")
            .step("step1")
            .timeout(Duration::from_secs(10))
            .done()
            .step("step2")
            .no_compensation()
            .done()
            .step("step3")
            .done()
            .build();

        assert_eq!(saga.saga_type, "test_saga");
        assert_eq!(saga.description, Some("A test saga".to_string()));
        assert_eq!(saga.step_count(), 3);

        let step1 = saga.get_step(0).unwrap();
        assert_eq!(step1.name, "step1");
        assert!(step1.requires_compensation);
        assert!(step1.timeout.is_some());

        let step2 = saga.get_step(1).unwrap();
        assert_eq!(step2.name, "step2");
        assert!(!step2.requires_compensation);

        let step3 = saga.get_step(2).unwrap();
        assert_eq!(step3.name, "step3");
        assert!(step3.requires_compensation);
    }

    #[tokio::test]
    async fn test_saga_state_persistence() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = SagaExecutor::new(store);

        let saga = SagaBuilder::new("persist_test").step("step1").done().build();

        // Start saga
        let state = executor.start_saga(saga, None).await.unwrap();
        assert!(matches!(state.state, SagaState::Executing { current_step: 0 }));

        // Load it back
        let loaded = executor.load_state(&state.execution_id).await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.execution_id, state.execution_id);
    }

    #[tokio::test]
    async fn test_saga_forward_execution() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = SagaExecutor::new(store);

        let saga = SagaBuilder::new("forward_test").step("step1").done().step("step2").done().build();

        let mut state = executor.start_saga(saga, None).await.unwrap();

        // Complete step 1
        executor.complete_step(&mut state, 0, Some("output1".to_string())).await.unwrap();
        assert!(matches!(state.state, SagaState::Executing { current_step: 1 }));

        // Complete step 2
        executor.complete_step(&mut state, 1, Some("output2".to_string())).await.unwrap();
        assert!(matches!(state.state, SagaState::Completed));
    }

    #[tokio::test]
    async fn test_saga_compensation() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = SagaExecutor::new(store);

        let saga = SagaBuilder::new("compensation_test")
            .step("step1")
            .done()
            .step("step2")
            .done()
            .step("step3")
            .done()
            .build();

        let mut state = executor.start_saga(saga, None).await.unwrap();

        // Complete steps 1 and 2
        executor.complete_step(&mut state, 0, None).await.unwrap();
        executor.complete_step(&mut state, 1, None).await.unwrap();

        // Fail step 3
        executor.fail_step(&mut state, 2, "step3 failed".to_string()).await.unwrap();

        // Should now be compensating, starting from step 2 (index 1)
        assert!(matches!(state.state, SagaState::Compensating {
            failed_step: 2,
            current_compensation: 1,
            ..
        }));

        // Complete compensation for step 2
        executor.complete_compensation(&mut state, 1, CompensationResult::Success).await.unwrap();

        // Should now be compensating step 1
        assert!(matches!(state.state, SagaState::Compensating {
            current_compensation: 0,
            ..
        }));

        // Complete compensation for step 1
        executor.complete_compensation(&mut state, 0, CompensationResult::Success).await.unwrap();

        // Should now be fully compensated
        assert!(matches!(state.state, SagaState::CompensationCompleted { .. }));
    }

    #[tokio::test]
    async fn test_get_next_action() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = SagaExecutor::new(store);

        let saga = SagaBuilder::new("action_test").step("step1").done().step("step2").done().build();

        let state = executor.start_saga(saga, None).await.unwrap();

        // Initially should execute step 0
        let action = executor.get_next_action(&state);
        assert_eq!(action, Some(SagaAction::Execute { step_index: 0 }));
    }

    #[tokio::test]
    async fn test_branch_backed_skip_compensation() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = SagaExecutor::new(store);

        let saga = SagaBuilder::new("branch_skip_test")
            .step("step1")
            .done()
            .step("step2")
            .branch_backed()
            .done()
            .step("step3")
            .done()
            .build();

        let mut state = executor.start_saga(saga, None).await.unwrap();

        // Complete steps 1 and 2
        executor.complete_step(&mut state, 0, None).await.unwrap();
        executor.complete_step(&mut state, 1, None).await.unwrap();

        // Fail step 3 — triggers compensation
        executor.fail_step(&mut state, 2, "step3 failed".into()).await.unwrap();

        // Step 2 is branch_backed — get_next_action should skip compensation
        let action = executor.get_next_action(&state);
        assert_eq!(action, Some(SagaAction::SkipCompensation { step_index: 1 }));
    }

    #[cfg(feature = "kv-branch")]
    #[tokio::test]
    async fn test_branch_backed_step_success() {
        use aspen_kv_types::WriteRequest;
        use aspen_traits::KeyValueStore;

        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = SagaExecutor::new(Arc::clone(&store));

        let saga = SagaBuilder::new("branch_success").step("branched_step").branch_backed().done().build();

        let mut state = executor.start_saga(saga, None).await.unwrap();

        // Run step inside a branch — writes a key
        executor
            .run_step_in_branch(&mut state, 0, |branch| async move {
                branch.write(WriteRequest::set("test-key", "test-value")).await.map_err(|e| e.to_string())?;
                Ok(Some("wrote test-key".into()))
            })
            .await
            .unwrap();

        // Saga should be completed
        assert!(matches!(state.state, SagaState::Completed));

        // The key should be in the base store (committed)
        let result = store.read(aspen_kv_types::ReadRequest::new("test-key")).await.unwrap();
        assert_eq!(result.kv.unwrap().value, "test-value");
    }

    #[cfg(feature = "kv-branch")]
    #[tokio::test]
    async fn test_branch_backed_step_failure_no_orphans() {
        use aspen_kv_types::WriteRequest;
        use aspen_traits::KeyValueStore;

        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = SagaExecutor::new(Arc::clone(&store));

        let saga = SagaBuilder::new("branch_fail").step("branched_step").branch_backed().done().build();

        let mut state = executor.start_saga(saga, None).await.unwrap();

        // Run step inside a branch — writes a key then fails
        executor
            .run_step_in_branch(&mut state, 0, |branch| async move {
                branch
                    .write(WriteRequest::set("orphan-key", "should-not-persist"))
                    .await
                    .map_err(|e| e.to_string())?;
                Err("simulated failure".into())
            })
            .await
            .unwrap();

        // Saga should be in compensation state (or completed compensation
        // since step 0 failed, there's nothing to compensate)
        assert!(matches!(state.state, SagaState::CompensationCompleted { .. }), "got {:?}", state.state);

        // The key should NOT be in the base store (branch was dropped)
        let result = store.read(aspen_kv_types::ReadRequest::new("orphan-key")).await;
        assert!(result.is_err() || result.unwrap().kv.is_none(), "orphan key should not exist in base store");
    }

    #[cfg(feature = "kv-branch")]
    #[tokio::test]
    async fn test_mixed_saga_branch_and_compensation() {
        use aspen_kv_types::WriteRequest;
        use aspen_traits::KeyValueStore;

        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = SagaExecutor::new(Arc::clone(&store));

        let saga = SagaBuilder::new("mixed_saga")
            .step("normal_step")
            .done() // requires compensation
            .step("branched_step")
            .branch_backed()
            .done() // branch-backed, no compensation needed
            .build();

        let mut state = executor.start_saga(saga, None).await.unwrap();

        // Complete normal step (writes directly to store)
        executor.complete_step(&mut state, 0, Some("step1 done".into())).await.unwrap();

        // Run branched step — succeeds
        executor
            .run_step_in_branch(&mut state, 1, |branch| async move {
                branch.write(WriteRequest::set("branch-key", "val")).await.map_err(|e| e.to_string())?;
                Ok(None)
            })
            .await
            .unwrap();

        assert!(matches!(state.state, SagaState::Completed));

        // branch-key should be committed
        let result = store.read(aspen_kv_types::ReadRequest::new("branch-key")).await.unwrap();
        assert_eq!(result.kv.unwrap().value, "val");
    }
}
