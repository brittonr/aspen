# Verification: Native callback value materialization

## Scope

The change starts from native-host commit `7fb4c9de293f815654e1e3b954f4fee3678ac87f`. Its lifecycle plan is commit `702adc9f3`.

The implementation publishes the exact v2 callback cohort:

- `molten.system-extension.native-host-profile.v2`;
- `molten.system-extension.native-callback-envelope.v2`;
- `molten.system-extension.native-callback-outcome.v2`;
- `molten/system-extension/native/v2`;
- `preserves-packed-materialized-values-v2`.

There is no v1 or reference-only fallback.

## Positive evidence

- Core profile and ingress tests admit exact v2 materialized values.
- Wire tests round-trip payload, prior state, output, effect request, next state, and checkpoint bytes.
- The value-port test verifies exact BLAKE3 materialization and publication.
- The separate-process service fixture runs activation, ingress, semantic-state replacement, effect-body publication, effect completion, checkpoint, restart, recovery, drain, stop, and removal.
- Durable memory and Redb journal tests round-trip independent semantic state and checkpoint references.
- Offline artifact verification includes semantic-state and value-publication members.
- Journal history proves callback intent precedes value-publication intent.

## Negative evidence

- Missing, corrupt, substituted, oversized, trailing, malformed, ambient-effect, and reference-only values deny.
- Legacy schema, ALPN, framing, and missing materialization requirements fail Nickel admission.
- Uncertain ingress publication remains unknown and does not start the callback.
- Uncertain callback publication leaves semantic state unchanged and releases no provider effects.
- Restart with missing state and checkpoint bytes fails before process start.
- Timeout, cancellation, nonzero exit, output flood, missing executable, stale generation, and transport drift remain denied.

## Checks

The following checks pass:

- `cargo fmt --all -- --check`;
- `cargo test --workspace`;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
- focused `molten-core` native-host tests;
- focused `molten` native-host unit and separate-process integration tests;
- `checks.x86_64-linux.native-system-extension-host-profile`;
- `checks.x86_64-linux.native-system-extension-octet-deny-all`.

The strict pinned Octet check reports `Status: clean`, zero findings, zero warnings, and zero errors for the native-host core and exact value port. Repository-wide `cargo octet check` remains warning-only with inherited broad-workspace findings, so it is not acceptance evidence.

Cairn validation plus proposal, design, and structural tasks gates pass with the current Cairn policy. Aspen's checked-in generated policy still contains the retired `task_marker_policy.markers` field, so validation uses the current Cairn repository policy.

Repository-wide Tracey still fails on inherited coverage gaps. None of the six `molten.system_extension.native_host.value_*` or `semantic_state` requirements appears in its missing or dangling sets after sync.

## Non-claims

The evidence does not prove value meaning, durable deployment storage, callback correctness, provider success, sandboxing, hermeticity, transport delivery, distributed availability, or release readiness.
