#[tokio::test]
async fn reusable_testing_core_helpers_work_without_cluster_bootstrap() {
    use std::time::Duration;

    use aspen_testing_core::DeterministicClusterController;
    use aspen_testing_core::DeterministicKeyValueStore;
    use aspen_testing_core::wait_until;

    let _kv = DeterministicKeyValueStore::new();
    let _cluster = DeterministicClusterController::new();

    wait_until("fixture is ready", Duration::from_secs(1), Duration::from_millis(5), || async { Ok(true) })
        .await
        .expect("wait helper should succeed");
}
