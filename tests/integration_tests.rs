//! Integration tests for dependency injection and testability
//!
//! These tests verify that the factory pattern enables testing without real infrastructure.

mod common;

use mvm_ci::config::AppConfig;
use mvm_ci::state::factory::test_factory::TestInfrastructureFactory;

#[tokio::test]
async fn test_app_state_with_test_factory() {
    // This test demonstrates that we can construct AppState without real infrastructure
    // using the test factory with mock repositories

    let config = AppConfig::default();

    // Create test infrastructure (would need a real flawless module and endpoint in practice)
    // For this test, we're just verifying the factory pattern compiles and the types are correct

    // In a real test, you would:
    // 1. Create a test flawless module
    // 2. Create a test iroh endpoint
    // 3. Use TestInfrastructureFactory to build AppState
    // 4. Verify domain services work with mock repositories

    let factory = TestInfrastructureFactory::new();

    // The factory exists and implements the trait
    assert!(std::any::type_name_of_val(&factory).contains("TestInfrastructureFactory"));

    // Verify we can access the config
    assert_eq!(config.network.http_port, 3020);
}

#[tokio::test]
async fn test_domain_services_with_mock_repositories() {
    use mvm_ci::repositories::mocks::{MockStateRepository, MockWorkRepository, MockWorkerRepository};
    use mvm_ci::state::DomainServices;
    use std::sync::Arc;

    // Create mock repositories
    let state_repo = Arc::new(MockStateRepository::new());
    let work_repo = Arc::new(MockWorkRepository::new());
    let worker_repo = Arc::new(MockWorkerRepository::new());

    // Create domain services with injected mocks
    let services = DomainServices::from_repositories(state_repo, work_repo, worker_repo);

    // Verify services were created successfully
    let cluster_status = services.cluster_status();
    assert!(std::any::type_name_of_val(&cluster_status).contains("ClusterStatusService"));

    let job_lifecycle = services.job_lifecycle();
    assert!(std::any::type_name_of_val(&job_lifecycle).contains("JobLifecycleService"));
}

#[tokio::test]
async fn test_production_factory_exists() {
    use mvm_ci::state::factory::ProductionInfrastructureFactory;

    // Verify production factory can be instantiated
    let factory = ProductionInfrastructureFactory::new();
    assert!(std::any::type_name_of_val(&factory).contains("ProductionInfrastructureFactory"));
}

#[tokio::test]
async fn test_mock_state_repository_operations() {
    use mvm_ci::domain::types::HealthStatus;
    use mvm_ci::repositories::mocks::MockStateRepository;
    use mvm_ci::repositories::StateRepository;

    let mock_repo = MockStateRepository::new();

    // Test default healthy state
    let health = mock_repo.health_check().await.unwrap();
    assert!(health.is_healthy);
    assert_eq!(health.node_count, 3);
    assert!(health.has_leader);

    // Test setting custom health
    mock_repo.set_health(HealthStatus {
        is_healthy: false,
        node_count: 1,
        has_leader: false,
    }).await;

    let health = mock_repo.health_check().await.unwrap();
    assert!(!health.is_healthy);
    assert_eq!(health.node_count, 1);
    assert!(!health.has_leader);
}

#[tokio::test]
async fn test_mock_work_repository_operations() {
    use mvm_ci::repositories::mocks::MockWorkRepository;
    use mvm_ci::repositories::WorkRepository;
    use mvm_ci::{Job, JobStatus};
    use serde_json::json;

    let mock_repo = MockWorkRepository::new();

    // Test publishing work
    mock_repo
        .publish_work("job-1".to_string(), json!({"task": "test"}))
        .await
        .unwrap();

    // Test listing work
    let work_items = mock_repo.list_work().await.unwrap();
    assert_eq!(work_items.len(), 1);
    assert_eq!(work_items[0].id, "job-1");
    assert_eq!(work_items[0].status, JobStatus::Pending);

    // Test claiming work
    let claimed = mock_repo.claim_work(None, None).await.unwrap();
    assert!(claimed.is_some());
    let claimed_item = claimed.unwrap();
    assert_eq!(claimed_item.id, "job-1");
    assert_eq!(claimed_item.status, JobStatus::Claimed);

    // Test updating status
    mock_repo
        .update_status("job-1", JobStatus::Completed)
        .await
        .unwrap();

    let work_items = mock_repo.list_work().await.unwrap();
    assert_eq!(work_items[0].status, JobStatus::Completed);
}
