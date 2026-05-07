# Native built-in host and redacted handle tests

- Change: `define-runtime-service-core`
- Task: tests proving built-in declarations use `NativeBuiltIn` and redacted capability handles
- Started: `2026-05-07T00:22:52Z`
- Completed: `2026-05-07T00:23:21Z`

## Implemented

Added `built_in_declarations_use_native_host_and_redacted_capability_handles` in `crates/aspen-runtime-core/src/lib.rs`.

The test proves:

- the built-in factory declaration uses `RuntimeHostKind::NativeBuiltIn`;
- the artifact is `RuntimeArtifact::BuiltIn`;
- capability receipt/public surfaces use `RedactedValue::OpaqueHandle("kv:forge")` rather than raw/plain diagnostic values;
- redacted handles do not trigger secret-shape detection;
- the declaration still carries the underlying capability binding handle/proof references for admission/projection rather than logging them as plain receipt diagnostics.

## Verification

```console
$ rustfmt crates/aspen-runtime-core/src/lib.rs
$ CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core --all-targets
running 7 tests
...
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
