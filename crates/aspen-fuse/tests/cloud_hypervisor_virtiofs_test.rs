//! Integration test: full VirtioFS data path through a real Cloud Hypervisor VM.
//!
//! ```text
//! Guest: echo > /mnt/file  →  virtio-fs device  →  vhost-user socket
//!        →  AspenVirtioFsHandler  →  AspenFs (in-memory BTreeMap)
//! ```
//!
//! This test proves the complete path works end-to-end: a real Linux guest
//! kernel mounts virtiofs, writes a file, reads it back, and verifies the
//! content — all served by our AspenVirtioFsHandler (not virtiofsd).
//!
//! After the VM exits, we also verify that the guest's write landed in the
//! in-memory BTreeMap backing the AspenFs, proving data flows all the way
//! from guest userspace into our KV store.
//!
//! Requires: KVM (`/dev/kvm`), cloud-hypervisor, and the test initramfs.
//! Environment variables (set by `nix develop`):
//!   - `CLOUD_HYPERVISOR_BIN` — path to cloud-hypervisor binary
//!   - `CH_KERNEL` — path to Linux bzImage
//!   - `VIRTIOFS_TEST_INITRD` — path to busybox initramfs with virtiofs test
#![cfg(feature = "virtiofs")]

use std::path::PathBuf;
use std::time::Duration;

use aspen_fuse::AspenFs;
use aspen_fuse::spawn_virtiofs_daemon;

/// Read an env var or skip the test with a message.
fn env_or_skip(var: &str) -> Option<String> {
    match std::env::var(var) {
        Ok(val) if !val.is_empty() => Some(val),
        _ => {
            eprintln!("skipping: {var} not set (run inside `nix develop`)");
            None
        }
    }
}

/// Full Cloud Hypervisor VirtioFS integration test.
///
/// Spawns our VirtioFS daemon backed by an in-memory AspenFs, boots a minimal
/// Linux guest via cloud-hypervisor, and verifies:
///   1. The guest can write/read files through the virtio-fs device
///   2. The written data actually landed in our in-memory KV store
#[tokio::test]
#[ignore] // Requires KVM + cloud-hypervisor; run with --run-ignored all
async fn test_cloud_hypervisor_virtiofs_file_io() {
    let ch_bin = match env_or_skip("CLOUD_HYPERVISOR_BIN") {
        Some(v) => v,
        None => return,
    };
    let kernel = match env_or_skip("CH_KERNEL") {
        Some(v) => v,
        None => return,
    };
    let initrd = match env_or_skip("VIRTIOFS_TEST_INITRD") {
        Some(v) => v,
        None => return,
    };

    // Verify KVM is available
    if !PathBuf::from("/dev/kvm").exists() {
        eprintln!("skipping: /dev/kvm not available");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let socket_path = tmp.path().join("virtiofs.sock");
    let serial_log = tmp.path().join("serial.log");
    let api_socket = tmp.path().join("api.sock");

    // Create in-memory filesystem with a shared handle to the backing store.
    // The test keeps `kv_store` to inspect contents after the VM writes.
    let (fs, kv_store) = AspenFs::new_in_memory_shared(0, 0);

    // Spawn our VirtioFS daemon on a background OS thread.
    // This creates the vhost-user socket and blocks in accept() until
    // cloud-hypervisor connects.
    let daemon_handle = spawn_virtiofs_daemon(&socket_path, fs).unwrap();

    // Poll until the socket file appears (daemon creates it in serve() -> Listener::new())
    for _ in 0..100_u32 {
        if socket_path.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(socket_path.exists(), "daemon failed to create socket within 1s");

    // Launch cloud-hypervisor with our VirtioFS socket.
    // The guest initramfs will:
    //   1. insmod virtio + fuse kernel modules
    //   2. mount -t virtiofs testfs /mnt
    //   3. Write "hello from guest" to /mnt/result.txt, read it back
    //   4. Print VIRTIOFS_TEST_PASS or VIRTIOFS_TEST_FAIL to serial console
    //   5. poweroff -f
    let mut child = tokio::process::Command::new(&ch_bin)
        .arg("--kernel")
        .arg(&kernel)
        .arg("--initramfs")
        .arg(&initrd)
        .arg("--cmdline")
        .arg("console=ttyS0 panic=1")
        .arg("--cpus")
        .arg("boot=1,max=1")
        .arg("--memory")
        .arg("size=256M,shared=on")
        .arg("--serial")
        .arg(format!("file={}", serial_log.display()))
        .arg("--console")
        .arg("off")
        .arg("--fs")
        .arg(format!("tag=testfs,socket={},num_queues=1,queue_size=512", socket_path.display()))
        .arg("--api-socket")
        .arg(format!("path={}", api_socket.display()))
        .spawn()
        .expect("failed to spawn cloud-hypervisor");

    // Wait for the guest to poweroff (the initramfs calls `poweroff -f`).
    // 30s timeout covers kernel boot + module loading + virtiofs mount + I/O.
    let exit_status = tokio::time::timeout(Duration::from_secs(30), child.wait())
        .await
        .expect("cloud-hypervisor did not exit within 30s")
        .expect("failed to wait on cloud-hypervisor");

    // Read serial console output
    let serial_output = std::fs::read_to_string(&serial_log).unwrap_or_default();

    // Print full serial log for debugging on failure
    if !serial_output.contains("VIRTIOFS_TEST_PASS") {
        eprintln!("--- serial log ---\n{serial_output}\n--- end serial log ---");
        eprintln!("cloud-hypervisor exit status: {exit_status}");
    }

    assert!(
        serial_output.contains("VIRTIOFS_TEST_PASS"),
        "guest did not report VIRTIOFS_TEST_PASS in serial output"
    );

    // Verify the guest's write actually landed in our in-memory KV store.
    // The guest wrote "hello from guest\n" to /mnt/result.txt, which maps
    // to KV key "result.txt" in the BTreeMap.
    {
        let store = kv_store.read().unwrap();
        let value = store.get("result.txt").expect("result.txt not found in KV store");
        let content = String::from_utf8_lossy(value);
        assert_eq!(content.trim(), "hello from guest", "KV store content mismatch: got {content:?}");

        // Verify we can enumerate keys
        let keys: Vec<&String> = store.keys().collect();
        assert!(keys.iter().any(|k| k.as_str() == "result.txt"), "result.txt not found in KV key listing: {keys:?}");
    }

    // Shut down the daemon. After cloud-hypervisor exits, the vhost-user
    // connection is closed, so serve() returns and the thread is joinable.
    daemon_handle.shutdown().unwrap();
}
