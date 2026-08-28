# Verification

## Baseline

Before the core change, `cargo test --test nativesystemextension` passed all six existing tests.

## Focused behavior

After implementation:

- `cargo test -p molten-core system_extension::native_host::tests::value` passed three tests.
- `cargo test --test nativesystemextension` passed eight tests.
- The positive test preserved exact provider output bytes in the version-two completion.
- The negative test rejected missing, identity-mismatched, and oversized output values.
- Rejected output value admission left the provider operation terminal and not retryable.

## Broad Rust checks

- `cargo test --workspace` passed all workspace, integration, and documentation tests.
- The main `molten` library passed 1,366 tests.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` passed.

One unrelated bounded-execution simulation test saw a transient broken pipe during an earlier parallel run. Its exact rerun passed, and the full workspace rerun passed.

## Nix and Octet

- `checks.x86_64-linux.native-system-extension-host-profile` passed.
- `checks.x86_64-linux.native-system-extension-octet-deny-all` passed with no accepted finding.
- The focused Octet workspace compiles the changed native-host core admission and positive and negative tests.

A repository-wide advisory `cargo octet check` still reports inherited warning-only findings. It is not used as acceptance evidence.

## Cairn

Proposal, design, tasks, sync, archive, and repository validation passed with the current generated Cairn policy.

Repository-wide Tracey still reports inherited gaps. The new requirement and all three scenarios have implementation and verification coverage.

## Claim boundary

This evidence proves exact bounded materialized provider completion bytes for the local native-host contract. It does not prove provider truth, durability, workload correctness, deployment readiness, or release eligibility.
