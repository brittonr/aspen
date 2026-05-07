# Native built-in service factory contract

- Change: `define-runtime-service-core`
- Task: linked native built-in service factory/manifest contract
- Started: `2026-05-07T00:21:29Z`
- Completed: `2026-05-07T00:22:28Z`

## Implemented

`crates/aspen-runtime-core/src/lib.rs` now models the native built-in boundary as a pure static-registration contract:

- `NativeLoadingPolicy::LinkedBuiltInOnly` is the only native built-in policy represented in portable runtime core.
- `NativeBuiltInServiceFactory` includes a `linked_symbol` and explicit `loading_policy` alongside the manifest.
- `admit_native_factory()` rejects name mismatches and empty linked-symbol handles.
- The contract has no path, library filename, filesystem, process, or dynamic loader field.

This keeps first-party native services as linked/static Aspen-node registrations and prevents this model slice from implying dynamic native plugin loading.

## Verification

```console
$ rustfmt crates/aspen-runtime-core/src/lib.rs
$ CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core --all-targets
running 6 tests
...
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
