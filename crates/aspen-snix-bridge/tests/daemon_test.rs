//! Integration tests for nix-daemon protocol support.
//!
//! These tests require:
//! - `snix-daemon` feature enabled
//! - `nix` binary on PATH
//! - Run with `--run-ignored all` since they need real nix CLI access
//!
//! ```bash
//! cargo nextest run -p aspen-snix-bridge --features snix-daemon --run-ignored all
//! ```

#![cfg(feature = "snix-daemon")]

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use aspen_snix::IrohBlobService;
use aspen_snix::RaftDirectoryService;
use aspen_snix::RaftPathInfoService;
use tokio::sync::watch;

/// Find a valid store path on the local system for testing.
/// Returns the store path string if one is found.
fn find_test_store_path() -> Option<String> {
    // Use a well-known path that's almost certainly in any nix store
    let output = Command::new("nix").args(["eval", "--raw", "nixpkgs#hello.outPath"]).output().ok()?;

    if output.status.success() {
        let path = String::from_utf8(output.stdout).ok()?;
        // Verify it exists
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }
    None
}

/// Build in-memory services for testing.
fn build_test_services() -> (
    Arc<dyn snix_castore::blobservice::BlobService>,
    Arc<dyn snix_castore::directoryservice::DirectoryService>,
    Arc<dyn snix_store::pathinfoservice::PathInfoService>,
) {
    let blob_store = aspen_blob::InMemoryBlobStore::new();
    let blob_svc: Arc<dyn snix_castore::blobservice::BlobService> = Arc::new(IrohBlobService::new(blob_store));

    let kv = aspen_testing::DeterministicKeyValueStore::new();
    let dir_svc: Arc<dyn snix_castore::directoryservice::DirectoryService> =
        Arc::new(RaftDirectoryService::from_arc(kv.clone()));
    let pathinfo_svc: Arc<dyn snix_store::pathinfoservice::PathInfoService> =
        Arc::new(RaftPathInfoService::from_arc(kv));

    (blob_svc, dir_svc, pathinfo_svc)
}

// =========================================================================
// Non-ignored tests (no nix CLI required)
// =========================================================================

/// Test that serve_daemon creates and cleans up the socket on shutdown.
#[tokio::test]
async fn test_daemon_socket_lifecycle() {
    let socket_path = PathBuf::from("/tmp/aspen-daemon-test-lifecycle.sock");
    let _ = tokio::fs::remove_file(&socket_path).await;

    let (blob_svc, dir_svc, pathinfo_svc) = build_test_services();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let daemon_path = socket_path.clone();
    let daemon_task = tokio::spawn(async move {
        aspen_snix_bridge::daemon::serve_daemon(&daemon_path, blob_svc, dir_svc, pathinfo_svc, shutdown_rx).await
    });

    // Give the daemon time to bind
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Socket should exist now
    assert!(socket_path.exists(), "socket should be created after startup");

    // Signal shutdown
    let _ = shutdown_tx.send(true);
    let result = tokio::time::timeout(Duration::from_secs(2), daemon_task).await;
    assert!(result.is_ok(), "daemon should shut down within 2s");
    let inner = result.unwrap();
    assert!(inner.is_ok(), "daemon task should not panic");
    assert!(inner.unwrap().is_ok(), "daemon should exit cleanly");

    // Socket should be cleaned up
    assert!(!socket_path.exists(), "socket should be removed after shutdown");
}

/// Test that serve_daemon removes a stale socket before binding.
#[tokio::test]
async fn test_daemon_removes_stale_socket() {
    let socket_path = PathBuf::from("/tmp/aspen-daemon-test-stale.sock");

    // Create a stale socket file
    tokio::fs::write(&socket_path, b"stale").await.unwrap();
    assert!(socket_path.exists());

    let (blob_svc, dir_svc, pathinfo_svc) = build_test_services();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let daemon_path = socket_path.clone();
    let daemon_task = tokio::spawn(async move {
        aspen_snix_bridge::daemon::serve_daemon(&daemon_path, blob_svc, dir_svc, pathinfo_svc, shutdown_rx).await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Should have replaced the stale file with a real socket
    assert!(socket_path.exists(), "socket should exist after removing stale file");

    let _ = shutdown_tx.send(true);
    let _ = tokio::time::timeout(Duration::from_secs(2), daemon_task).await;
    let _ = tokio::fs::remove_file(&socket_path).await;
}

// =========================================================================
// Ignored tests (require nix CLI on PATH)
// =========================================================================

/// 6.10: Test `nix path-info --store unix:///tmp/aspen-daemon.sock` against Aspen store.
///
/// With an empty store, querying any path should report "not valid".
#[tokio::test]
#[ignore = "requires nix CLI on PATH"]
async fn test_daemon_path_info_empty_store() {
    let socket_path = PathBuf::from("/tmp/aspen-daemon-test-pathinfo.sock");
    let (blob_svc, dir_svc, pathinfo_svc) = build_test_services();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let daemon_path = socket_path.clone();
    let daemon_task = tokio::spawn(async move {
        aspen_snix_bridge::daemon::serve_daemon(&daemon_path, blob_svc, dir_svc, pathinfo_svc, shutdown_rx).await
    });

    // Give the daemon time to bind
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Query a path that doesn't exist in the store
    let output = Command::new("nix")
        .args([
            "path-info",
            "--store",
            &format!("unix://{}", socket_path.display()),
            "/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-hello-2.12.1",
        ])
        .output()
        .expect("failed to run nix path-info");

    // Should fail (path not in store)
    assert!(!output.status.success(), "expected failure for non-existent path, got success");

    let _ = shutdown_tx.send(true);
    let _ = tokio::time::timeout(Duration::from_secs(2), daemon_task).await;
    let _ = tokio::fs::remove_file(&socket_path).await;
}

/// 6.11: Test `nix copy --to unix:///tmp/aspen-daemon.sock` uploads a store path.
///
/// Copies a store path into the daemon's store, then queries it back.
#[tokio::test]
#[ignore = "requires nix CLI on PATH and a store path to copy"]
async fn test_daemon_copy_and_query() {
    let Some(test_path) = find_test_store_path() else {
        eprintln!("skipping: no test store path found (need nixpkgs#hello)");
        return;
    };

    let socket_path = PathBuf::from("/tmp/aspen-daemon-test-copy.sock");
    let (blob_svc, dir_svc, pathinfo_svc) = build_test_services();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let daemon_path = socket_path.clone();
    let daemon_task = tokio::spawn(async move {
        aspen_snix_bridge::daemon::serve_daemon(&daemon_path, blob_svc, dir_svc, pathinfo_svc, shutdown_rx).await
    });

    // Give the daemon time to bind
    tokio::time::sleep(Duration::from_millis(200)).await;

    let store_url = format!("unix://{}", socket_path.display());

    // Copy a store path to the daemon
    let copy_output = Command::new("nix")
        .args(["copy", "--to", &store_url, &test_path])
        .output()
        .expect("failed to run nix copy");

    assert!(copy_output.status.success(), "nix copy failed: {}", String::from_utf8_lossy(&copy_output.stderr));

    // Now query it back — should succeed
    let query_output = Command::new("nix")
        .args(["path-info", "--store", &store_url, &test_path])
        .output()
        .expect("failed to run nix path-info");

    assert!(
        query_output.status.success(),
        "nix path-info failed after copy: {}",
        String::from_utf8_lossy(&query_output.stderr)
    );

    let _ = shutdown_tx.send(true);
    let _ = tokio::time::timeout(Duration::from_secs(2), daemon_task).await;
    let _ = tokio::fs::remove_file(&socket_path).await;
}
