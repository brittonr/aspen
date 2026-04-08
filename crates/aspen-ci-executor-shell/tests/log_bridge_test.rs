//! Test that `log_bridge` captures stderr lines into the KV log store.

use std::sync::Arc;

use aspen_ci_executor_shell::CI_LOG_COMPLETE_MARKER;
use aspen_ci_executor_shell::CI_LOG_KV_PREFIX;
use aspen_ci_executor_shell::log_bridge;
use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;

#[tokio::test]
async fn test_stderr_captured_in_kv_log_store() {
    let kv = aspen_testing::DeterministicKeyValueStore::new();
    let (tx, rx) = tokio::sync::mpsc::channel::<String>(100);

    let run_id = "run-001".to_string();
    let job_id = "job-001".to_string();

    // Spawn the bridge
    let kv_clone: Arc<dyn KeyValueStore> = kv.clone();
    let rid = run_id.clone();
    let jid = job_id.clone();
    let handle = tokio::spawn(async move {
        log_bridge(rx, kv_clone, rid, jid).await;
    });

    // Simulate stderr lines from a fast-failing build
    let lines = vec![
        "error: flake 'path:.' has no attribute 'checks'\n",
        "       Did you mean one of: check?\n",
        "       at «none»:0: (source not available)\n",
    ];
    for line in &lines {
        tx.send(line.to_string()).await.unwrap();
    }

    // Close channel to signal completion
    drop(tx);
    handle.await.unwrap();

    // Verify: at least one log chunk was written
    let chunk_key = format!("{CI_LOG_KV_PREFIX}{run_id}:{job_id}:{:010}", 0);
    let result = kv.read(ReadRequest::new(&chunk_key)).await;
    assert!(result.is_ok(), "log chunk 0 should exist in KV");

    let kv_entry = result.unwrap().kv.expect("log chunk should have a value");
    let stored = &kv_entry.value;
    // The chunk should contain the stderr content
    for line in &lines {
        assert!(
            stored.contains(line.trim()),
            "KV log chunk missing stderr line: {line}"
        );
    }

    // Verify: completion marker was written
    let marker_key = format!("{CI_LOG_KV_PREFIX}{run_id}:{job_id}:{CI_LOG_COMPLETE_MARKER}");
    let marker_result = kv.read(ReadRequest::new(&marker_key)).await;
    assert!(marker_result.is_ok(), "completion marker should exist in KV");

    let marker_value = marker_result.unwrap().kv.expect("marker should have a value");
    let marker_json: serde_json::Value =
        serde_json::from_str(&marker_value.value).unwrap();
    assert_eq!(marker_json["status"], "done");
    assert!(marker_json["total_chunks"].as_u64().unwrap() >= 1);
}
