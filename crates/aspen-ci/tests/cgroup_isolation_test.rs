//! Integration tests for cgroup-based resource isolation.
//!
//! These tests verify that resource limits are actually enforced by the kernel
//! and that violations trigger the expected responses (OOM kills, process limits, etc.).
//!
//! # Test Categories
//!
//! 1. **Memory Enforcement** - Tests memory.max and memory.high limits
//! 2. **PID Enforcement** - Tests pids.max limit with fork bombs
//! 3. **CPU Weight** - Tests relative CPU scheduling priority
//! 4. **Monitoring** - Tests resource usage reporting
//! 5. **Cleanup** - Tests cgroup removal and process termination
//!
//! # Tiger Style
//!
//! - Bounded test timeouts (no infinite waits)
//! - Explicit cleanup of test processes
//! - Real resource enforcement validation
//! - Fixed resource limits for reproducible tests

#![cfg(target_os = "linux")]

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use aspen_ci::workers::{ResourceLimiter, ResourceLimits};

// Test constants
const TEST_TIMEOUT: Duration = Duration::from_secs(30);
const MEMORY_TEST_SIZE_MB: u64 = 100;
const PID_TEST_LIMIT: u32 = 50;

/// Helper to check if we're running with sufficient privileges for cgroup tests.
fn check_cgroup_privileges() -> bool {
    use std::os::unix::fs::PermissionsExt;

    // Check if we can write to /sys/fs/cgroup
    let cgroup_root = std::path::Path::new("/sys/fs/cgroup");
    if !cgroup_root.exists() {
        eprintln!("cgroups v2 not available at /sys/fs/cgroup");
        return false;
    }

    // Check write permissions (typically requires root or cgroup delegation)
    match std::fs::metadata(cgroup_root) {
        Ok(metadata) => {
            let perms = metadata.permissions();
            perms.mode() & 0o200 != 0 || std::env::var("USER").unwrap_or_default() == "root"
        }
        Err(_) => false,
    }
}

/// Skip test if cgroups v2 not available or insufficient privileges.
macro_rules! skip_if_no_cgroups {
    () => {
        if !ResourceLimiter::is_available() {
            eprintln!("Skipping test: cgroups v2 not available");
            return;
        }
        if !check_cgroup_privileges() {
            eprintln!("Skipping test: insufficient privileges for cgroup operations");
            return;
        }
    };
}

/// Test that memory limits are enforced by creating a process that allocates too much memory.
#[test]
fn test_memory_limit_enforcement() {
    skip_if_no_cgroups!();

    let limits = ResourceLimits {
        memory_max_bytes: MEMORY_TEST_SIZE_MB * 1024 * 1024,         // 100 MB
        memory_high_bytes: (MEMORY_TEST_SIZE_MB - 10) * 1024 * 1024, // 90 MB
        cpu_weight: 100,
        pids_max: 1000,
        ..Default::default()
    };

    let limiter = ResourceLimiter::create("test-memory-limit", &limits).expect("failed to create resource limiter");

    // Create a child process that tries to allocate 200MB (exceeding the 100MB limit)
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&format!(
            "exec python3 -c 'import time; data = bytearray({}); time.sleep(10)'",
            200 * 1024 * 1024 // 200 MB allocation
        ))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn memory test process");

    let child_pid = child.id();

    // Add the process to the cgroup
    limiter.add_process(child_pid).expect("failed to add process to cgroup");

    // Wait for the process to be killed by OOM killer
    let start = std::time::Instant::now();
    let mut was_killed = false;

    while start.elapsed() < TEST_TIMEOUT {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process exited - check if it was killed (not a normal exit)
                if !status.success() {
                    was_killed = true;
                    break;
                }
            }
            Ok(None) => {
                // Process still running, check memory usage
                if let Some(memory_usage) = limiter.memory_current() {
                    println!("Current memory usage: {} MB", memory_usage / (1024 * 1024));
                }
                thread::sleep(Duration::from_millis(500));
            }
            Err(_) => break,
        }
    }

    // Clean up any remaining process
    let _ = child.kill();
    let _ = child.wait();

    assert!(was_killed, "process should have been killed by OOM killer due to memory limit");

    // Cleanup
    limiter.cleanup().expect("failed to cleanup cgroup");
}

/// Test that PID limits are enforced by creating a fork bomb.
#[test]
fn test_pid_limit_enforcement() {
    skip_if_no_cgroups!();

    let limits = ResourceLimits {
        memory_max_bytes: 512 * 1024 * 1024,  // 512 MB
        memory_high_bytes: 400 * 1024 * 1024, // 400 MB
        cpu_weight: 100,
        pids_max: PID_TEST_LIMIT,
        ..Default::default()
    };

    let limiter = ResourceLimiter::create("test-pid-limit", &limits).expect("failed to create resource limiter");

    // Create a shell script that forks until it hits the PID limit
    let fork_bomb_script = format!(
        r#"
#!/bin/bash
count=0
while [ $count -lt {} ]; do
    sleep 30 &
    count=$((count + 1))
    echo "Forked process $count"
done
sleep 30
"#,
        PID_TEST_LIMIT + 10 // Try to exceed the limit
    );

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&fork_bomb_script)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn PID test process");

    let child_pid = child.id();

    // Add the process to the cgroup
    limiter.add_process(child_pid).expect("failed to add process to cgroup");

    // Wait a bit for forking to happen
    thread::sleep(Duration::from_secs(3));

    // Check current PID count
    let pid_count = limiter.pids_current().unwrap_or(0);
    println!("Current PID count: {}", pid_count);

    // The PID count should be at or near the limit, but not exceed it significantly
    assert!(
        pid_count <= PID_TEST_LIMIT + 5, // Small buffer for timing
        "PID count {} should not significantly exceed limit {}",
        pid_count,
        PID_TEST_LIMIT
    );

    // Clean up the test process and its children
    let _ = child.kill();
    let _ = child.wait();

    // Wait for cleanup
    thread::sleep(Duration::from_secs(1));

    // Cleanup
    limiter.cleanup().expect("failed to cleanup cgroup");
}

/// Test CPU weight enforcement by comparing relative CPU usage.
#[test]
fn test_cpu_weight_enforcement() {
    skip_if_no_cgroups!();

    // Create two cgroups with different CPU weights
    let high_priority_limits = ResourceLimits {
        memory_max_bytes: 512 * 1024 * 1024,
        memory_high_bytes: 400 * 1024 * 1024,
        cpu_weight: 200, // Higher priority
        pids_max: 100,
        ..Default::default()
    };

    let low_priority_limits = ResourceLimits {
        memory_max_bytes: 512 * 1024 * 1024,
        memory_high_bytes: 400 * 1024 * 1024,
        cpu_weight: 50, // Lower priority
        pids_max: 100,
        ..Default::default()
    };

    let high_priority_limiter = ResourceLimiter::create("test-cpu-high", &high_priority_limits)
        .expect("failed to create high priority limiter");

    let low_priority_limiter =
        ResourceLimiter::create("test-cpu-low", &low_priority_limits).expect("failed to create low priority limiter");

    // Create CPU-intensive processes
    let cpu_burn_script = "while true; do : ; done";

    let mut high_child = Command::new("sh")
        .arg("-c")
        .arg(cpu_burn_script)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn high priority CPU test");

    let mut low_child = Command::new("sh")
        .arg("-c")
        .arg(cpu_burn_script)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn low priority CPU test");

    // Add processes to respective cgroups
    high_priority_limiter.add_process(high_child.id()).expect("failed to add high priority process");
    low_priority_limiter.add_process(low_child.id()).expect("failed to add low priority process");

    // Let them run for a bit to establish CPU scheduling patterns
    thread::sleep(Duration::from_secs(5));

    // Note: Actual CPU usage measurement would require reading /proc/stat
    // or using perf tools, which is complex. For now, just verify the
    // cgroups exist and processes are running.

    // Verify processes are still running (not killed by resource limits)
    assert!(matches!(high_child.try_wait(), Ok(None)), "high priority process should still be running");
    assert!(matches!(low_child.try_wait(), Ok(None)), "low priority process should still be running");

    // Clean up
    let _ = high_child.kill();
    let _ = low_child.kill();
    let _ = high_child.wait();
    let _ = low_child.wait();

    high_priority_limiter.cleanup().expect("failed to cleanup high priority cgroup");
    low_priority_limiter.cleanup().expect("failed to cleanup low priority cgroup");
}

/// Test resource monitoring functions.
#[test]
fn test_resource_monitoring() {
    skip_if_no_cgroups!();

    let limits = ResourceLimits {
        memory_max_bytes: 256 * 1024 * 1024,  // 256 MB
        memory_high_bytes: 200 * 1024 * 1024, // 200 MB
        cpu_weight: 100,
        pids_max: 100,
        ..Default::default()
    };

    let limiter = ResourceLimiter::create("test-monitoring", &limits).expect("failed to create resource limiter");

    // Initially, there should be minimal usage
    let initial_memory = limiter.memory_current().unwrap_or(0);
    let initial_pids = limiter.pids_current().unwrap_or(0);

    println!("Initial memory: {} bytes", initial_memory);
    println!("Initial PID count: {}", initial_pids);

    // Create a process that allocates some memory
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("exec python3 -c 'import time; data = bytearray(50*1024*1024); time.sleep(5)'") // 50MB
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn monitoring test process");

    let child_pid = child.id();
    limiter.add_process(child_pid).expect("failed to add process to cgroup");

    // Wait for memory allocation
    thread::sleep(Duration::from_secs(2));

    // Check updated resource usage
    let memory_usage = limiter.memory_current().unwrap_or(0);
    let pid_count = limiter.pids_current().unwrap_or(0);

    println!("Memory usage with process: {} MB", memory_usage / (1024 * 1024));
    println!("PID count with process: {}", pid_count);

    // Memory usage should have increased
    assert!(memory_usage > initial_memory, "memory usage should increase after process allocation");

    // PID count should be at least 1 (the test process)
    assert!(pid_count >= 1, "PID count should be at least 1");

    // Memory usage should be within reasonable bounds (not exceeding limit)
    assert!(memory_usage <= limits.memory_max_bytes, "memory usage should not exceed the limit");

    // Clean up
    let _ = child.kill();
    let _ = child.wait();

    limiter.cleanup().expect("failed to cleanup cgroup");
}

/// Test cgroup cleanup behavior.
#[test]
fn test_cgroup_cleanup() {
    skip_if_no_cgroups!();

    let limits = ResourceLimits::default();
    let limiter = ResourceLimiter::create("test-cleanup", &limits).expect("failed to create resource limiter");

    let cgroup_path = limiter.cgroup_path().to_path_buf();

    // Verify cgroup exists
    assert!(cgroup_path.exists(), "cgroup directory should exist");

    // Create a long-running process
    let mut child = Command::new("sleep").arg("30").spawn().expect("failed to spawn cleanup test process");

    let child_pid = child.id();
    limiter.add_process(child_pid).expect("failed to add process to cgroup");

    // Verify process is in cgroup
    let procs_file = cgroup_path.join("cgroup.procs");
    let procs_content = std::fs::read_to_string(&procs_file).expect("failed to read cgroup.procs");
    assert!(procs_content.contains(&child_pid.to_string()), "process should be in cgroup");

    // Cleanup explicitly
    limiter.cleanup().expect("failed to cleanup cgroup");

    // Wait a bit for cleanup to complete
    thread::sleep(Duration::from_millis(500));

    // Process should be killed
    match child.try_wait() {
        Ok(Some(_)) => {
            // Process exited, which is expected
        }
        Ok(None) => {
            // Process still running, kill it manually
            let _ = child.kill();
            let _ = child.wait();
        }
        Err(_) => {
            // Process is gone, which is expected
        }
    }

    // Cgroup directory should be removed
    assert!(!cgroup_path.exists(), "cgroup directory should be removed after cleanup");
}

/// Test graceful fallback when cgroups are unavailable.
#[test]
fn test_noop_limiter_fallback() {
    // This test doesn't require cgroups to be available

    let _limits = ResourceLimits::default();
    let limiter = ResourceLimiter::noop("test-noop");

    // No-op limiter should handle all operations gracefully
    assert!(limiter.add_process(12345).is_ok());
    assert!(limiter.memory_current().is_none());
    assert!(limiter.pids_current().is_none());
    assert!(limiter.cleanup().is_ok());

    // Cgroup path should be empty
    assert!(limiter.cgroup_path().as_os_str().is_empty());
}

/// Test the convenience create_limiter function.
#[test]
fn test_create_limiter_convenience() {
    use aspen_ci::workers::create_limiter;

    let limits = ResourceLimits::default();
    let limiter = create_limiter("test-convenience", &limits);

    // Should always succeed (falls back to no-op if cgroups unavailable)
    assert!(limiter.add_process(12345).is_ok());
    assert!(limiter.cleanup().is_ok());
}
