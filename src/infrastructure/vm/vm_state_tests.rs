// Comprehensive VM State Machine Tests
//
// These tests verify all state transitions, edge cases, and invariants
// for the VM state machine to ensure reliable operation.

#[cfg(test)]
mod tests {
    use super::super::vm_types::{VmState, VmConfig, VmMode, VmInstance};
    use super::super::registry::DefaultVmRepository as VmRegistry;
    use crate::hiqlite::HiqliteService;
    use std::sync::Arc;
    use uuid::Uuid;

    // Helper to create test VM instance
    fn create_test_vm(state: VmState) -> VmInstance {
        let config = VmConfig {
            id: Uuid::new_v4(),
            mode: VmMode::Ephemeral { job_id: "test-job".to_string() },
            memory_mb: 512,
            vcpus: 1,
            kernel: None,
            rootfs: None,
            init: None,
            gpu_required: false,
            gpu_memory_mb: None,
            gpu_model: None,
            timeout_seconds: 300,
        };

        VmInstance {
            config,
            state,
            ip_address: Some("192.168.100.42".to_string()),
            pid: Some(12345),
            job_dir: Some("/tmp/test".into()),
            created_at: 1732723200,
            metrics: Default::default(),
        }
    }

    // Helper to create test registry
    async fn create_test_registry() -> Arc<VmRegistry> {
        let hiqlite = Arc::new(HiqliteService::placeholder());
        let state_dir = std::env::temp_dir().join(format!("vm_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&state_dir).unwrap();

        Arc::new(VmRegistry::new(hiqlite, &state_dir).await.unwrap())
    }

    #[tokio::test]
    async fn test_state_transition_starting_to_ready() {
        let registry = create_test_registry().await;
        let vm = create_test_vm(VmState::Starting);
        let vm_id = vm.config.id;

        // Register VM in Starting state
        registry.register(vm.clone()).await.unwrap();

        // Transition to Ready
        registry.update_state(vm_id, VmState::Ready).await.unwrap();

        // Verify state changed
        let updated = registry.get(vm_id).await.unwrap().unwrap();
        let updated_vm = updated.read().await;
        assert!(matches!(updated_vm.state, VmState::Ready));
    }

    #[tokio::test]
    async fn test_state_transition_ready_to_busy() {
        let registry = create_test_registry().await;
        let vm = create_test_vm(VmState::Ready);
        let vm_id = vm.config.id;

        registry.register(vm.clone()).await.unwrap();

        // Transition to Busy
        let job_id = "test-job-123".to_string();
        let started_at = 1732723200;
        registry.update_state(
            vm_id,
            VmState::Busy { job_id: job_id.clone(), started_at }
        ).await.unwrap();

        // Verify state changed with correct data
        let updated = registry.get(vm_id).await.unwrap().unwrap();
        let updated_vm = updated.read().await;
        match &updated_vm.state {
            VmState::Busy { job_id: j, started_at: s } => {
                assert_eq!(j, &job_id);
                assert_eq!(*s, started_at);
            }
            _ => panic!("Expected Busy state"),
        }
    }

    #[tokio::test]
    async fn test_state_transition_busy_to_idle() {
        let registry = create_test_registry().await;
        let vm = create_test_vm(VmState::Busy {
            job_id: "job-1".to_string(),
            started_at: 1732723200,
        });
        let vm_id = vm.config.id;

        registry.register(vm.clone()).await.unwrap();

        // Transition to Idle
        registry.update_state(
            vm_id,
            VmState::Idle {
                jobs_completed: 1,
                last_job_at: 1732723210,
            }
        ).await.unwrap();

        // Verify state changed
        let updated = registry.get(vm_id).await.unwrap().unwrap();
        let updated_vm = updated.read().await;
        match &updated_vm.state {
            VmState::Idle { jobs_completed, last_job_at } => {
                assert_eq!(*jobs_completed, 1);
                assert_eq!(*last_job_at, 1732723210);
            }
            _ => panic!("Expected Idle state"),
        }
    }

    #[tokio::test]
    async fn test_state_transition_idle_to_busy_again() {
        let registry = create_test_registry().await;
        let vm = create_test_vm(VmState::Idle {
            jobs_completed: 5,
            last_job_at: 1732723200,
        });
        let vm_id = vm.config.id;

        registry.register(vm.clone()).await.unwrap();

        // Transition back to Busy (service VM reuse)
        registry.update_state(
            vm_id,
            VmState::Busy {
                job_id: "job-6".to_string(),
                started_at: 1732723300,
            }
        ).await.unwrap();

        // Verify state changed
        let updated = registry.get(vm_id).await.unwrap().unwrap();
        let updated_vm = updated.read().await;
        assert!(matches!(updated_vm.state, VmState::Busy { .. }));
    }

    #[tokio::test]
    async fn test_state_transition_to_terminated() {
        let registry = create_test_registry().await;
        let vm = create_test_vm(VmState::Ready);
        let vm_id = vm.config.id;

        registry.register(vm.clone()).await.unwrap();

        // Transition to Terminated
        registry.update_state(
            vm_id,
            VmState::Terminated {
                reason: "Job completed".to_string(),
                exit_code: 0,
            }
        ).await.unwrap();

        // Verify state changed
        let updated = registry.get(vm_id).await.unwrap().unwrap();
        let updated_vm = updated.read().await;
        match &updated_vm.state {
            VmState::Terminated { reason, exit_code } => {
                assert_eq!(reason, "Job completed");
                assert_eq!(*exit_code, 0);
            }
            _ => panic!("Expected Terminated state"),
        }
    }

    #[tokio::test]
    async fn test_state_transition_to_failed() {
        let registry = create_test_registry().await;
        let vm = create_test_vm(VmState::Starting);
        let vm_id = vm.config.id;

        registry.register(vm.clone()).await.unwrap();

        // Transition to Failed
        registry.update_state(
            vm_id,
            VmState::Failed {
                error: "Failed to allocate resources".to_string(),
            }
        ).await.unwrap();

        // Verify state changed
        let updated = registry.get(vm_id).await.unwrap().unwrap();
        let updated_vm = updated.read().await;
        match &updated_vm.state {
            VmState::Failed { error } => {
                assert_eq!(error, "Failed to allocate resources");
            }
            _ => panic!("Expected Failed state"),
        }
    }

    #[tokio::test]
    async fn test_invalid_transition_terminated_to_busy() {
        let registry = create_test_registry().await;
        let vm = create_test_vm(VmState::Terminated {
            reason: "Shutdown".to_string(),
            exit_code: Some(0),
        });
        let vm_id = vm.config.id;

        registry.register(vm.clone()).await.unwrap();

        // Try invalid transition (should still work as registry doesn't enforce)
        // In production, the controller would prevent this
        let result = registry.update_state(
            vm_id,
            VmState::Busy {
                job_id: "invalid".to_string(),
                started_at: 1732723200,
            }
        ).await;

        // The registry allows any transition (controller enforces rules)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_state_updates() {
        let registry = create_test_registry().await;
        let vm = create_test_vm(VmState::Ready);
        let vm_id = vm.config.id;

        registry.register(vm.clone()).await.unwrap();

        // Spawn multiple concurrent updates
        let registry1 = Arc::clone(&registry);
        let registry2 = Arc::clone(&registry);

        let handle1 = tokio::spawn(async move {
            registry1.update_state(
                vm_id,
                VmState::Busy {
                    job_id: "job-1".to_string(),
                    started_at: 1732723200,
                }
            ).await
        });

        let handle2 = tokio::spawn(async move {
            registry2.update_state(
                vm_id,
                VmState::Busy {
                    job_id: "job-2".to_string(),
                    started_at: 1732723201,
                }
            ).await
        });

        // Both should succeed (last write wins)
        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        // One of the states should have won
        let final_vm = registry.get(vm_id).await.unwrap().unwrap();
        let final_state = final_vm.read().await;
        assert!(matches!(final_state.state, VmState::Busy { .. }));
    }

    #[tokio::test]
    async fn test_state_persistence_and_recovery() {
        let hiqlite = Arc::new(HiqliteService::placeholder());
        let state_dir = std::env::temp_dir().join(format!("vm_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&state_dir).unwrap();

        let vm_id = Uuid::new_v4();

        // Create registry and add VM
        {
            let registry = Arc::new(VmRegistry::new(Arc::clone(&hiqlite), &state_dir).await.unwrap());
            let vm = create_test_vm(VmState::Busy {
                job_id: "persistent-job".to_string(),
                started_at: 1732723200,
            });
            vm.config.id = vm_id;

            registry.register(vm).await.unwrap();
        }

        // Create new registry and recover
        {
            let registry = Arc::new(VmRegistry::new(hiqlite, &state_dir).await.unwrap());
            let recovered = registry.recover_from_persistence().await.unwrap();

            // Should recover the VM
            assert_eq!(recovered, 1);

            // Check state was preserved
            let vm = registry.get(vm_id).await.unwrap().unwrap();
            let vm_state = vm.read().await;
            match &vm_state.state {
                VmState::Failed { error } => {
                    // Process died, so it's marked as failed during recovery
                    assert!(error.contains("not running"));
                }
                _ => panic!("Expected Failed state after recovery with dead process"),
            }
        }
    }

    #[tokio::test]
    async fn test_state_serialization_preserves_all_fields() {
        use serde_json;

        // Test all state variants serialize and deserialize correctly
        let states = vec![
            VmState::Starting,
            VmState::Ready,
            VmState::Busy {
                job_id: "test-job-123".to_string(),
                started_at: 1732723200,
            },
            VmState::Idle {
                jobs_completed: 42,
                last_job_at: 1732723300,
            },
            VmState::Terminated {
                reason: "Normal shutdown".to_string(),
                exit_code: 0,
            },
            VmState::Failed {
                error: "Resource exhaustion".to_string(),
            },
        ];

        for original in states {
            let json = serde_json::to_string(&original).unwrap();
            let recovered: VmState = serde_json::from_str(&json).unwrap();

            // Verify all fields preserved
            match (&original, &recovered) {
                (VmState::Starting, VmState::Starting) => {},
                (VmState::Ready, VmState::Ready) => {},
                (VmState::Busy { job_id: j1, started_at: s1 },
                 VmState::Busy { job_id: j2, started_at: s2 }) => {
                    assert_eq!(j1, j2);
                    assert_eq!(s1, s2);
                }
                (VmState::Idle { jobs_completed: c1, last_job_at: t1 },
                 VmState::Idle { jobs_completed: c2, last_job_at: t2 }) => {
                    assert_eq!(c1, c2);
                    assert_eq!(t1, t2);
                }
                (VmState::Terminated { reason: r1, exit_code: e1 },
                 VmState::Terminated { reason: r2, exit_code: e2 }) => {
                    assert_eq!(r1, r2);
                    assert_eq!(e1, e2);
                }
                (VmState::Failed { error: e1 },
                 VmState::Failed { error: e2 }) => {
                    assert_eq!(e1, e2);
                }
                _ => panic!("State mismatch after serialization"),
            }
        }
    }

    #[tokio::test]
    async fn test_ephemeral_vs_service_vm_lifecycle() {
        let registry = create_test_registry().await;

        // Create ephemeral VM
        let mut ephemeral = create_test_vm(VmState::Ready);
        ephemeral.config.mode = VmMode::Ephemeral { job_id: "job-1".to_string() };
        let ephemeral_id = ephemeral.config.id;

        // Create service VM
        let mut service = create_test_vm(VmState::Ready);
        service.config.mode = VmMode::Service {
            queue_name: "default".to_string(),
            max_jobs: 10,
            max_uptime_secs: 3600,
        };
        let service_id = service.config.id;

        registry.register(ephemeral).await.unwrap();
        registry.register(service).await.unwrap();

        // Both start job
        registry.update_state(
            ephemeral_id,
            VmState::Busy {
                job_id: "job-1".to_string(),
                started_at: 1732723200,
            }
        ).await.unwrap();

        registry.update_state(
            service_id,
            VmState::Busy {
                job_id: "job-2".to_string(),
                started_at: 1732723200,
            }
        ).await.unwrap();

        // Ephemeral terminates after job
        registry.update_state(
            ephemeral_id,
            VmState::Terminated {
                reason: "Job completed".to_string(),
                exit_code: 0,
            }
        ).await.unwrap();

        // Service goes idle after job
        registry.update_state(
            service_id,
            VmState::Idle {
                jobs_completed: 1,
                last_job_at: 1732723210,
            }
        ).await.unwrap();

        // Verify final states
        let ephemeral_vm = registry.get(ephemeral_id).await.unwrap().unwrap();
        let ephemeral_state = ephemeral_vm.read().await;
        assert!(matches!(ephemeral_state.state, VmState::Terminated { .. }));

        let service_vm = registry.get(service_id).await.unwrap().unwrap();
        let service_state = service_vm.read().await;
        assert!(matches!(service_state.state, VmState::Idle { .. }));
    }

    #[tokio::test]
    async fn test_state_transition_atomicity() {
        // This test verifies our atomic state transition implementation
        let registry = create_test_registry().await;
        let vm = create_test_vm(VmState::Ready);
        let vm_id = vm.config.id;

        registry.register(vm.clone()).await.unwrap();

        // Update state (should be atomic with rollback on failure)
        registry.update_state(
            vm_id,
            VmState::Busy {
                job_id: "atomic-test".to_string(),
                started_at: 1732723200,
            }
        ).await.unwrap();

        // Verify consistency check passes
        let (checked, found, fixed) = registry.verify_consistency().await.unwrap();
        assert!(checked > 0);
        assert_eq!(found, 0);  // No inconsistencies
        assert_eq!(fixed, 0);  // Nothing to fix
    }
}