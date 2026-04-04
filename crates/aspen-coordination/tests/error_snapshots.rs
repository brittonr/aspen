//! Snapshot tests for coordination error message formatting.
//!
//! Ensures error messages remain stable and actionable.
//! Run `cargo insta review` to approve changes.

use aspen_coordination::CoordinationError;

#[test]
fn snapshot_lock_held() {
    let err = CoordinationError::LockHeld {
        holder: "worker-42".to_string(),
        deadline_ms: 1_700_000_000_000,
    };
    insta::assert_snapshot!("lock_held", err.to_string());
}

#[test]
fn snapshot_lock_lost() {
    let err = CoordinationError::LockLost {
        expected_holder: "worker-1".to_string(),
        current_holder: "worker-2".to_string(),
    };
    insta::assert_snapshot!("lock_lost", err.to_string());
}

#[test]
fn snapshot_timeout() {
    let err = CoordinationError::Timeout {
        operation: "acquire lock 'my-lock'".to_string(),
    };
    insta::assert_snapshot!("timeout", err.to_string());
}

#[test]
fn snapshot_max_retries() {
    let err = CoordinationError::MaxRetriesExceeded {
        operation: "CAS write to 'counter:main'".to_string(),
        attempts: 10,
    };
    insta::assert_snapshot!("max_retries", err.to_string());
}
