use std::collections::HashSet;

use super::types::WorkerInfo;
use crate::registry::HealthStatus;
use crate::types::now_unix_ms;

#[test]
fn test_worker_capacity() {
    let mut worker = WorkerInfo {
        worker_id: "w1".to_string(),
        node_id: "n1".to_string(),
        peer_id: None,
        capabilities: vec![],
        load: 0.3,
        active_jobs: 3,
        max_concurrent: 10,
        queue_depth: 5,
        health: HealthStatus::Healthy,
        tags: vec![],
        last_heartbeat_ms: now_unix_ms(),
        started_at_ms: now_unix_ms(),
        total_processed: 100,
        total_failed: 2,
        avg_processing_time_ms: 50,
        groups: HashSet::new(),
    };

    assert_eq!(worker.available_capacity(), 0.7);

    worker.health = HealthStatus::Unhealthy;
    assert_eq!(worker.available_capacity(), 0.0);
}

#[test]
fn test_worker_can_handle() {
    let worker = WorkerInfo {
        worker_id: "w1".to_string(),
        node_id: "n1".to_string(),
        peer_id: None,
        capabilities: vec!["email".to_string(), "sms".to_string()],
        load: 0.5,
        active_jobs: 5,
        max_concurrent: 10,
        queue_depth: 0,
        health: HealthStatus::Healthy,
        tags: vec![],
        last_heartbeat_ms: now_unix_ms(),
        started_at_ms: now_unix_ms(),
        total_processed: 0,
        total_failed: 0,
        avg_processing_time_ms: 0,
        groups: HashSet::new(),
    };

    assert!(worker.can_handle("email"));
    assert!(worker.can_handle("sms"));
    assert!(!worker.can_handle("push"));
}
