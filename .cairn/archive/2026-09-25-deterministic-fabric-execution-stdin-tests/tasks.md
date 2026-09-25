# Tasks: Make the fabric_execution stdin tests deterministic

## Phase 1: Implementation

- [x] [serial] Add no-input request helpers and stop passing input to non-reading live children. a[deterministic-fabric-execution-stdin-tests.no-unread-input] a[deterministic-fabric-execution-stdin-tests.subjects-kept]
- [x] [serial] Make the composition child read its input with the builtin `read`. a[deterministic-fabric-execution-stdin-tests.no-unread-input]
- [x] [serial] Add the oversized-input pinning test, and document that the input-delivery follow-up revises it. a[deterministic-fabric-execution-stdin-tests.conservative-pin]

## Phase 2: Validation

- [x] [serial] Run the `fabric_execution::` tests 200 consecutive times, serial and parallel, and record the logs. a[deterministic-fabric-execution-stdin-tests.repeatable]
- [x] [serial] Build `checks.x86_64-linux.nextest`, and run `cargo fmt --check`, workspace clippy, and `cargo test --workspace`. a[deterministic-fabric-execution-stdin-tests.repeatable] a[deterministic-fabric-execution-stdin-tests.subjects-kept]
