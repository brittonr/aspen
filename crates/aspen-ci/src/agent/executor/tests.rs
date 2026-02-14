//! Tests for the executor module.

use std::path::Path;

use tokio::sync::oneshot;

use super::*;

#[tokio::test]
async fn test_validate_working_dir_rejects_outside_workspace() {
    let executor = Executor::new();

    let result = executor.validate_working_dir(Path::new("/tmp/evil"));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("/workspace"));
}

#[tokio::test]
async fn test_validate_working_dir_rejects_root() {
    let executor = Executor::new();

    let result = executor.validate_working_dir(Path::new("/"));
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_working_dir_rejects_relative_path() {
    let executor = Executor::new();

    let result = executor.validate_working_dir(Path::new("workspace/project"));
    assert!(result.is_err());
}

#[tokio::test]
async fn test_executor_is_running_empty() {
    let executor = Executor::new();
    assert!(!executor.is_running("nonexistent-job").await);
}

#[tokio::test]
async fn test_cancel_nonexistent_job() {
    let executor = Executor::new();

    let result = executor.cancel("nonexistent-job").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test]
async fn test_job_handle_cancel() {
    let (tx, rx) = oneshot::channel();
    let handle = JobHandle { cancel_tx: tx };

    // Cancel should send signal
    handle.cancel();

    // Receiver should get the signal
    assert!(rx.await.is_ok());
}

#[test]
fn test_executor_default() {
    let executor = Executor::default();
    // Just verify it can be created via Default
    assert!(std::ptr::eq(&executor as *const _, &executor as *const _));
}
