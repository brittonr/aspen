Evidence-ID: transport-neutral-raft-types.warning-behavior
Task-ID: V2
Artifact-Type: warning-transcript
Covers: transport.transport-neutral-raft-membership-metadata.runtime-conversion-failure-is-surfaced-explicitly

# Warning Behavior Verification

This transcript proves one invalid conversion path identifies the failing member in a warning and returns an unavailable outcome (`leader_info=None`).

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
warning: struct `RedbTestNode` is never constructed
  --> crates/aspen-raft/src/node/tests.rs:98:8
   |
98 | struct RedbTestNode {
   |        ^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: function `create_test_redb_node_at_path` is never used
   --> crates/aspen-raft/src/node/tests.rs:104:10
    |
104 | async fn create_test_redb_node_at_path(
    |          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: function `create_test_redb_node` is never used
   --> crates/aspen-raft/src/node/tests.rs:137:10
    |
137 | async fn create_test_redb_node(
    |          ^^^^^^^^^^^^^^^^^^^^^

warning: `aspen-raft` (lib test) generated 3 warnings
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.55s
     Running unittests src/lib.rs (target/debug/deps/aspen_raft-a90f828ae41f49a6)

running 1 test
captured_warning=WARN cannot use current leader membership address because it is invalid leader_id=1 endpoint_id=not-a-public-key error=invalid node address not-a-public-key: invalid endpoint id: not-a-public-key
leader_info=None
test node::warning_tests::test_current_leader_info_warns_and_returns_none_for_invalid_leader_address ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 784 filtered out; finished in 0.00s

