# Runtime service model invariant tests

- Change: `define-runtime-service-core`
- Task: model invariant tests
- Started: `2026-05-07T00:19:21Z`
- Completed: `2026-05-07T00:20:48Z`

## Implemented

Added pure admission helpers and tests in `crates/aspen-runtime-core/src/lib.rs` covering:

- service identity (`RuntimeApplicationRef.service_id`) cannot be blank;
- desired replica count cannot be zero;
- singleton services must declare exactly one replica;
- route declarations must be owned by the service they are attached to;
- health policy interval/timeout/threshold bounds;
- restart policy window/backoff bounds;
- upgrade policy `max_unavailable` bound against desired replicas;
- host-loading reference compatibility through `RuntimeServiceSpec::as_unit_declaration()` and existing host/artifact admission;
- lifecycle transition adjacency, with direct declared-to-running rejected;
- healthy instance receipts that remain redaction-safe.

## Verification

```console
$ rustfmt crates/aspen-runtime-core/src/lib.rs
$ CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core --all-targets
running 6 tests
...
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
