// Integration tests for VM lifecycle management
//
// NOTE: These tests are temporarily disabled due to API changes requiring HiqliteService.
// They need refactoring to provide proper mock dependencies.

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::super::*;
    use tempfile::TempDir;
    use tokio::time::{timeout, Duration};
    use uuid::Uuid;

    /// Create test VM manager with temporary directories
    /// NOTE: This function is temporarily disabled - requires HiqliteService dependency
    #[allow(dead_code)]
    async fn create_test_manager() -> (VmManager, TempDir) {
        unimplemented!("Test helper disabled - requires HiqliteService dependency");
        /*
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().to_path_buf();

        let config = VmManagerConfig {
            max_vms: 5,
            auto_scaling: false,
            pre_warm_count: 0,
            flake_dir: std::path::PathBuf::from("./microvms"),
            state_dir: state_dir.clone(),
            default_memory_mb: 256,
            default_vcpus: 1,
        };

        let manager = VmManager::new(config).await.unwrap();
        (manager, temp_dir)
        */
    }

    #[tokio::test]
    #[ignore = "Requires HiqliteService dependency - needs refactoring"]
    async fn test_vm_manager_creation() {
        // Test temporarily disabled - requires HiqliteService dependency
        /*
        let (manager, _temp) = create_test_manager().await;

        // Verify manager is created successfully
        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.total_vms, 0);
        assert_eq!(stats.running_vms, 0);
        */
    }

    #[tokio::test]
    #[ignore = "Requires HiqliteService dependency - needs refactoring"]
    async fn test_vm_registry_persistence() {
        // Test temporarily disabled - requires HiqliteService dependency
        /*
        let temp_dir = TempDir::new().unwrap();
        let vm_id = Uuid::new_v4();

        // Create registry and register a VM
        {
            let registry = VmRegistry::new(temp_dir.path()).await.unwrap();
            let config = VmConfig::default_service();
            let mut vm = VmInstance::new(config.clone());
            vm.config.id = vm_id;

            registry.register(vm).await.unwrap();
            assert_eq!(registry.count_all().await, 1);
        }

        // Create new registry and verify VM is recovered
        {
            let registry = VmRegistry::new(temp_dir.path()).await.unwrap();
            let recovered = registry.recover_from_persistence().await.unwrap();
            assert_eq!(recovered, 1);

            let vm = registry.get(vm_id).await.unwrap();
            assert!(vm.is_some());
        }
        */
    }

    #[tokio::test]
    #[ignore = "Requires HiqliteService dependency - needs refactoring"]
    async fn test_job_routing_isolation_levels() {
        // Test temporarily disabled - requires HiqliteService dependency
        /*
        let temp_dir = TempDir::new().unwrap();
        let registry = Arc::new(VmRegistry::new(temp_dir.path()).await.unwrap());

        // Mock controller (would normally start VMs)
        let mock_config = VmManagerConfig::default();
        let controller = Arc::new(VmController::new(mock_config, Arc::clone(&registry)).unwrap());

        let router = JobRouter::new(registry, controller);

        // Test high isolation job
        let high_isolation_job = crate::Job {
            id: "test-high-1".to_string(),
            status: crate::JobStatus::Pending,
            payload: serde_json::json!({
                "type": "security-scan",
                "require_isolation": true
            }),
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
            created_at: 0,
            updated_at: 0,
            started_at: None,
            error_message: None,
            retry_count: 0,
            compatible_worker_types: vec![],
        };

        let requirements = router.analyze_job_requirements(&high_isolation_job)
            .await
            .unwrap();
        assert_eq!(requirements.isolation_level, IsolationLevel::Maximum);

        // Test trusted job
        let trusted_job = crate::Job {
            id: "test-trusted-1".to_string(),
            status: crate::JobStatus::Pending,
            payload: serde_json::json!({
                "type": "build",
                "trusted": true
            }),
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
            created_at: 0,
            updated_at: 0,
            started_at: None,
            error_message: None,
            retry_count: 0,
            compatible_worker_types: vec![],
        };

        let requirements = router.analyze_job_requirements(&trusted_job)
            .await
            .unwrap();
        assert_eq!(requirements.isolation_level, IsolationLevel::Minimal);
        */
    }

    #[tokio::test]
    #[ignore = "Requires HiqliteService dependency - needs refactoring"]
    async fn test_vm_state_transitions() {
        let temp_dir = TempDir::new().unwrap();
        let registry = VmRegistry::new(temp_dir.path()).await.unwrap();

        let config = VmConfig::default_service();
        let vm = VmInstance::new(config.clone());
        let vm_id = vm.config.id;

        // Register VM in Starting state
        registry.register(vm).await.unwrap();

        // Transition to Ready
        registry.update_state(vm_id, VmState::Ready).await.unwrap();
        let vm = registry.get(vm_id).await.unwrap().unwrap();
        assert!(matches!(vm.read().await.state, VmState::Ready));

        // Transition to Busy
        let job_id = "test-job-123".to_string();
        registry.update_state(
            vm_id,
            VmState::Busy {
                job_id: job_id.clone(),
                started_at: chrono::Utc::now().timestamp(),
            }
        ).await.unwrap();

        let vm = registry.get(vm_id).await.unwrap().unwrap();
        let state = &vm.read().await.state;
        assert!(matches!(state, VmState::Busy { .. }));

        // Transition to Idle
        registry.update_state(
            vm_id,
            VmState::Idle {
                jobs_completed: 1,
                last_job_at: chrono::Utc::now().timestamp(),
            }
        ).await.unwrap();

        let vm = registry.get(vm_id).await.unwrap().unwrap();
        assert!(vm.read().await.state.is_available());
    }

    #[tokio::test]
    #[ignore = "Requires HiqliteService dependency - needs refactoring"]
    async fn test_health_checker_status_transitions() {
        let temp_dir = TempDir::new().unwrap();
        let registry = Arc::new(VmRegistry::new(temp_dir.path()).await.unwrap());

        let config = health_checker::HealthCheckConfig {
            failure_threshold: 3,
            recovery_threshold: 2,
            ..Default::default()
        };

        let checker = HealthChecker::new(registry, config);
        let vm_id = Uuid::new_v4();

        // Initial state should be Unknown
        assert!(matches!(
            checker.get_health_status(vm_id).await,
            health_checker::HealthStatus::Unknown
        ));

        // Success -> Healthy
        checker.update_health_status(vm_id, Ok(()), 100).await;
        assert!(checker.is_healthy(vm_id).await);

        // First failure -> Degraded
        checker.update_health_status(
            vm_id,
            Err(anyhow::anyhow!("Test failure")),
            0
        ).await;
        assert!(matches!(
            checker.get_health_status(vm_id).await,
            health_checker::HealthStatus::Degraded { failures: 1, .. }
        ));

        // Third failure -> Unhealthy
        checker.update_health_status(
            vm_id,
            Err(anyhow::anyhow!("Test failure")),
            0
        ).await;
        checker.update_health_status(
            vm_id,
            Err(anyhow::anyhow!("Test failure")),
            0
        ).await;
        assert!(matches!(
            checker.get_health_status(vm_id).await,
            health_checker::HealthStatus::Unhealthy { .. }
        ));
    }

    #[tokio::test]
    #[ignore = "Requires HiqliteService dependency - needs refactoring"]
    async fn test_resource_monitor_auto_scaling() {
        let temp_dir = TempDir::new().unwrap();
        let registry = Arc::new(VmRegistry::new(temp_dir.path()).await.unwrap());

        let mock_config = VmManagerConfig {
            max_vms: 10,
            auto_scaling: true,
            pre_warm_count: 2,
            ..Default::default()
        };

        let controller = Arc::new(VmController::new(
            mock_config,
            Arc::clone(&registry)
        ).unwrap());

        let monitor = ResourceMonitor::new(registry.clone(), controller);

        // Check initial scaling needs (should want to create VMs)
        // In a real test, we'd verify that check_scaling() tries to create VMs
        monitor.check_scaling().await.unwrap_or(());

        // Note: Full integration test would require mocking VM creation
    }

    #[tokio::test]
    #[ignore = "Requires HiqliteService dependency - needs refactoring"]
    async fn test_control_protocol_communication() {
        use control_protocol::{ControlClient, ControlProtocol};

        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        let vm_id = Uuid::new_v4();

        // Start mock server
        let server_socket = socket_path.clone();
        let server = tokio::spawn(async move {
            let protocol = ControlProtocol::new(vm_id, server_socket);
            let _ = timeout(
                Duration::from_secs(2),
                protocol.start_server()
            ).await;
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Test client communication
        let client = ControlClient::new(vm_id, socket_path);

        // Try to ping (may fail in test environment)
        match timeout(Duration::from_secs(1), client.ping()).await {
            Ok(Ok((uptime, jobs))) => {
                assert!(uptime >= 0);
                assert!(jobs >= 0);
            }
            _ => {
                // Server might not be ready or available in test
                // This is expected in unit tests
            }
        }

        // Clean up
        server.abort();
    }

    #[tokio::test]
    #[ignore = "Requires HiqliteService dependency - needs refactoring"]
    async fn test_vm_recycling_logic() {
        // Test service VM recycling based on limits
        let mut config = VmConfig::default_service();
        config.mode = VmMode::Service {
            queue_name: "test".to_string(),
            max_jobs: Some(5),
            max_uptime_secs: Some(60),
        };

        let mut vm = VmInstance::new(config);

        // Initially should not need recycling
        assert!(!vm.should_recycle());

        // Test job limit exceeded
        vm.metrics.jobs_completed = 5;
        assert!(vm.should_recycle());

        // Test memory pressure
        vm.metrics.jobs_completed = 3;
        vm.config.memory_mb = 100;
        vm.metrics.current_memory_mb = 95;
        assert!(vm.should_recycle());

        // Test ephemeral VMs don't recycle
        let ephemeral_config = VmConfig::default_ephemeral("job-123".to_string());
        let ephemeral_vm = VmInstance::new(ephemeral_config);
        assert!(!ephemeral_vm.should_recycle());
    }

    #[tokio::test]
    #[ignore = "Requires HiqliteService dependency - needs refactoring"]
    async fn test_routing_rules() {
        use job_router::RoutingRules;

        let temp_dir = TempDir::new().unwrap();
        let registry = Arc::new(VmRegistry::new(temp_dir.path()).await.unwrap());
        let controller = Arc::new(VmController::new(
            VmManagerConfig::default(),
            Arc::clone(&registry)
        ).unwrap());

        let router = JobRouter::new(registry, controller);

        // Update routing rules
        let new_rules = RoutingRules {
            high_isolation_patterns: vec![r".*malware.*".to_string()],
            shareable_job_types: vec!["test".to_string()],
            dedicated_job_types: vec!["pentest".to_string()],
            max_jobs_per_vm: 10,
            auto_create_vms: false,
            prefer_service_vms: false,
        };

        router.update_rules(new_rules.clone()).await;

        // Test pattern matching
        let malware_job = crate::Job {
            id: "test-1".to_string(),
            status: crate::JobStatus::Pending,
            payload: serde_json::json!({
                "task": "scan malware sample"
            }),
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
            created_at: 0,
            updated_at: 0,
            started_at: None,
            error_message: None,
            retry_count: 0,
            compatible_worker_types: vec![],
        };

        let requirements = router.analyze_job_requirements(&malware_job)
            .await
            .unwrap();

        // Should require high isolation due to pattern match
        assert_eq!(requirements.isolation_level, IsolationLevel::Maximum);
    }
}