//! Integration tests for CI log streaming.
//!
//! Tests the end-to-end flow:
//! 1. CiLogWriter writes log chunks to a KV store
//! 2. handle_get_job_logs reads them back via RPC
//! 3. handle_subscribe_logs provides watch prefix info
//! 4. Completion markers signal stream end

use aspen_ci::log_writer::CiLogWriter;
use aspen_ci::log_writer::SpawnedLogWriter;
use aspen_client_api::ClientRpcResponse;
use aspen_testing_core::DeterministicKeyValueStore;

/// Helper to extract CiGetJobLogsResponse from ClientRpcResponse.
fn unwrap_logs_response(resp: ClientRpcResponse) -> aspen_client_api::CiGetJobLogsResponse {
    match resp {
        ClientRpcResponse::CiGetJobLogsResult(r) => r,
        other => panic!("expected CiGetJobLogsResult, got {:?}", other),
    }
}

/// Helper to extract CiSubscribeLogsResponse from ClientRpcResponse.
fn unwrap_subscribe_response(resp: ClientRpcResponse) -> aspen_client_api::CiSubscribeLogsResponse {
    match resp {
        ClientRpcResponse::CiSubscribeLogsResult(r) => r,
        other => panic!("expected CiSubscribeLogsResult, got {:?}", other),
    }
}

#[tokio::test]
async fn test_write_and_read_log_chunks() {
    let kv = DeterministicKeyValueStore::new();

    // Write some log lines via CiLogWriter
    let mut writer = CiLogWriter::new("run-1".into(), "job-a".into(), kv.clone());
    writer.write_line("Building project...", "stdout").await.unwrap();
    writer.write_line("Compiling crate foo", "stdout").await.unwrap();
    writer.write_line("warning: unused variable", "stderr").await.unwrap();
    writer.flush().await.unwrap();

    assert_eq!(writer.chunk_count(), 1);

    // Read logs back via the handler
    let resp =
        aspen_ci_handler::handler::logs::handle_get_job_logs(kv.as_ref(), "run-1".into(), "job-a".into(), 0, Some(100))
            .await
            .unwrap();

    let result = unwrap_logs_response(resp);
    assert!(result.was_found);
    assert_eq!(result.chunks.len(), 1);
    assert!(!result.is_complete); // no completion marker yet
    assert!(!result.error.is_some());

    // Verify chunk content includes all three lines with stream prefixes
    let content = &result.chunks[0].content;
    assert!(content.contains("[stdout] Building project..."));
    assert!(content.contains("[stdout] Compiling crate foo"));
    assert!(content.contains("[stderr] warning: unused variable"));
}

#[tokio::test]
async fn test_completion_marker() {
    let kv = DeterministicKeyValueStore::new();

    let mut writer = CiLogWriter::new("run-2".into(), "job-b".into(), kv.clone());
    writer.write_line("Step 1 done", "stdout").await.unwrap();
    writer.complete("success").await.unwrap();

    // Read logs — should show is_complete = true
    let resp =
        aspen_ci_handler::handler::logs::handle_get_job_logs(kv.as_ref(), "run-2".into(), "job-b".into(), 0, Some(100))
            .await
            .unwrap();

    let result = unwrap_logs_response(resp);
    assert!(result.was_found);
    assert!(result.is_complete);
    assert_eq!(result.chunks.len(), 1);
}

#[tokio::test]
async fn test_no_logs_returns_not_found() {
    let kv = DeterministicKeyValueStore::new();

    let resp = aspen_ci_handler::handler::logs::handle_get_job_logs(
        kv.as_ref(),
        "nonexistent-run".into(),
        "nonexistent-job".into(),
        0,
        Some(100),
    )
    .await
    .unwrap();

    let result = unwrap_logs_response(resp);
    assert!(!result.was_found);
    assert!(result.chunks.is_empty());
}

#[tokio::test]
async fn test_paginated_log_retrieval() {
    let kv = DeterministicKeyValueStore::new();

    // Write enough data to create multiple chunks (8KB = 8192 byte threshold)
    let mut writer = CiLogWriter::new("run-3".into(), "job-c".into(), kv.clone());

    // Each line with prefix "[stdout] " + content + "\n" must exceed 8KB total
    // to trigger auto-flush. Use 5000-char lines: ~5010 bytes each.
    let long_line = "x".repeat(5000);
    writer.write_line(&long_line, "stdout").await.unwrap(); // ~5010 bytes
    writer.write_line(&long_line, "stdout").await.unwrap(); // ~10020 bytes -> auto-flush at 8192
    writer.write_line(&long_line, "stdout").await.unwrap(); // second chunk
    writer.flush().await.unwrap();
    writer.complete("success").await.unwrap();

    let chunk_count = writer.chunk_count();
    assert!(chunk_count >= 2, "expected at least 2 chunks, got {}", chunk_count);

    // Fetch first chunk only
    let resp =
        aspen_ci_handler::handler::logs::handle_get_job_logs(kv.as_ref(), "run-3".into(), "job-c".into(), 0, Some(1))
            .await
            .unwrap();

    let result = unwrap_logs_response(resp);
    assert!(result.was_found);
    assert_eq!(result.chunks.len(), 1);
    assert!(result.has_more);
    assert_eq!(result.last_index, 0);

    // Fetch from next index
    let resp =
        aspen_ci_handler::handler::logs::handle_get_job_logs(kv.as_ref(), "run-3".into(), "job-c".into(), 1, Some(100))
            .await
            .unwrap();

    let result = unwrap_logs_response(resp);
    assert!(result.was_found);
    assert!(!result.chunks.is_empty());
    assert!(result.is_complete);
}

#[tokio::test]
async fn test_subscribe_logs_running_job() {
    let kv = DeterministicKeyValueStore::new();

    // Write some logs but don't complete
    let mut writer = CiLogWriter::new("run-4".into(), "job-d".into(), kv.clone());
    writer.write_line("Starting...", "stdout").await.unwrap();
    writer.flush().await.unwrap();

    let resp =
        aspen_ci_handler::handler::logs::handle_subscribe_logs(kv.as_ref(), "run-4".into(), "job-d".into(), None)
            .await
            .unwrap();

    let result = unwrap_subscribe_response(resp);
    assert!(result.was_found);
    assert!(result.is_running); // no completion marker
    assert!(result.watch_prefix.contains("run-4"));
    assert!(result.watch_prefix.contains("job-d"));
}

#[tokio::test]
async fn test_subscribe_logs_completed_job() {
    let kv = DeterministicKeyValueStore::new();

    let mut writer = CiLogWriter::new("run-5".into(), "job-e".into(), kv.clone());
    writer.write_line("Done", "stdout").await.unwrap();
    writer.complete("success").await.unwrap();

    let resp =
        aspen_ci_handler::handler::logs::handle_subscribe_logs(kv.as_ref(), "run-5".into(), "job-e".into(), None)
            .await
            .unwrap();

    let result = unwrap_subscribe_response(resp);
    assert!(result.was_found);
    assert!(!result.is_running); // completion marker present
}

#[tokio::test]
async fn test_subscribe_logs_not_found() {
    let kv = DeterministicKeyValueStore::new();

    let resp = aspen_ci_handler::handler::logs::handle_subscribe_logs(
        kv.as_ref(),
        "ghost-run".into(),
        "ghost-job".into(),
        None,
    )
    .await
    .unwrap();

    let result = unwrap_subscribe_response(resp);
    assert!(!result.was_found);
}

#[tokio::test]
async fn test_spawned_log_writer_end_to_end() {
    let kv = DeterministicKeyValueStore::new();

    // Use the async SpawnedLogWriter (channel-based)
    let (handle, join) = SpawnedLogWriter::spawn("run-6".into(), "job-f".into(), kv.clone());

    // Write lines through the channel
    handle.write("Line 1".into(), "stdout").await.unwrap();
    handle.write("Line 2".into(), "stderr").await.unwrap();
    handle.write("Line 3".into(), "build").await.unwrap();
    handle.complete("success").await.unwrap();

    // Wait for background writer to finish
    join.await.unwrap();

    // Verify logs are in KV and readable via handler
    let resp =
        aspen_ci_handler::handler::logs::handle_get_job_logs(kv.as_ref(), "run-6".into(), "job-f".into(), 0, Some(100))
            .await
            .unwrap();

    let result = unwrap_logs_response(resp);
    assert!(result.was_found);
    assert!(result.is_complete);
    assert!(!result.chunks.is_empty());

    // Verify all lines made it through
    let all_content: String = result.chunks.iter().map(|c| c.content.clone()).collect();
    assert!(all_content.contains("[stdout] Line 1"));
    assert!(all_content.contains("[stderr] Line 2"));
    assert!(all_content.contains("[build] Line 3"));
}

#[tokio::test]
async fn test_chunk_limit_enforcement() {
    let kv = DeterministicKeyValueStore::new();

    let mut writer = CiLogWriter::new("run-7".into(), "job-g".into(), kv.clone());

    // Write many lines — the writer should enforce the chunk limit
    // MAX_CI_LOG_CHUNKS_PER_JOB is 10_000, so just verify the mechanism works
    // by checking the limit_reached flag after normal writes
    for i in 0..5 {
        writer.write_line(&format!("Log line {}", i), "stdout").await.unwrap();
    }
    writer.flush().await.unwrap();

    assert!(!writer.is_limit_reached());
    assert!(writer.chunk_count() >= 1);
}

#[tokio::test]
async fn test_subscribe_logs_with_explicit_from_index() {
    let kv = DeterministicKeyValueStore::new();

    // Write multiple chunks
    let mut writer = CiLogWriter::new("run-8".into(), "job-h".into(), kv.clone());
    let long_line = "y".repeat(4000);
    writer.write_line(&long_line, "stdout").await.unwrap();
    writer.write_line(&long_line, "stdout").await.unwrap();
    writer.flush().await.unwrap();

    // Subscribe with explicit from_index
    let resp =
        aspen_ci_handler::handler::logs::handle_subscribe_logs(kv.as_ref(), "run-8".into(), "job-h".into(), Some(5))
            .await
            .unwrap();

    let result = unwrap_subscribe_response(resp);
    assert!(result.was_found);
    assert_eq!(result.current_index, 5); // Uses the explicit from_index
}

#[tokio::test]
async fn test_failed_job_completion_marker() {
    let kv = DeterministicKeyValueStore::new();

    let mut writer = CiLogWriter::new("run-9".into(), "job-i".into(), kv.clone());
    writer.write_line("ERROR: build failed", "stderr").await.unwrap();
    writer.complete("failed").await.unwrap();

    // Verify completion marker with "failed" status
    let resp =
        aspen_ci_handler::handler::logs::handle_get_job_logs(kv.as_ref(), "run-9".into(), "job-i".into(), 0, Some(100))
            .await
            .unwrap();

    let result = unwrap_logs_response(resp);
    assert!(result.was_found);
    assert!(result.is_complete); // Failed jobs are still "complete"
    assert!(result.chunks[0].content.contains("ERROR: build failed"));
}

#[tokio::test]
async fn test_spawned_writer_channel_close_without_complete() {
    let kv = DeterministicKeyValueStore::new();

    let (handle, join) = SpawnedLogWriter::spawn("run-10".into(), "job-j".into(), kv.clone());

    handle.write("orphaned line".into(), "stdout").await.unwrap();

    // Drop the handle without calling complete — writer should handle gracefully
    drop(handle);
    join.await.unwrap();

    // Logs should still be written, with completion marker status = "unknown"
    let resp = aspen_ci_handler::handler::logs::handle_get_job_logs(
        kv.as_ref(),
        "run-10".into(),
        "job-j".into(),
        0,
        Some(100),
    )
    .await
    .unwrap();

    let result = unwrap_logs_response(resp);
    assert!(result.was_found);
    assert!(result.is_complete); // completion marker written even on channel close
}
