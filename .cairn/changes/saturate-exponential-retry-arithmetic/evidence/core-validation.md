# F12 core validation

## Result and scope

The bounded arithmetic repair passes the current core suite. This result does not complete F12 or establish release acceptance.

- Immediate outcome: admitted exponential retries saturate before high bits can disappear.
- Durable capability: repository tests cover arithmetic boundaries and unchanged admission rules.
- Maintenance owner: Molten fabric-time maintainers.
- Repetition evidence: exact commands, compressed logs, and `core-inputs.b3` in this directory.

The `.log.gz` files preserve the raw command output. `core-log-payloads.b3` identifies each uncompressed transcript. Compression preserves terminal blank lines that the whitespace gate rejects in tracked text files.

The implementation worktree starts at `1a390a0da2dc62f98566b2e55617da819cf549a4`. The helper reuses same-repository source from `brittonr/aspen` commit `fbd9ca63f5ce68a5b2007793b09913347468d68f`. The reused scope is `capped_exponential_delay` and its call from `plan_retry`. The new test module also adds a wide-integer reference and further rejection controls. No historical candidate receipt serves as current acceptance evidence.

## Observed checks

| Check | Result |
|---|---|
| Existing retry baseline before core edits | 1 passed, exit 0 |
| Existing fabric-time fixture baseline before core edits | 16 library tests and 2 CLI tests passed, exit 0 |
| New regressions before the repair | 6 passed, 4 failed, exit 101 |
| Full core suite after the final arithmetic tests | 370 passed, none failed or ignored, exit 0 |
| Core Clippy, all targets and all features, with `-D warnings` | Exit 0 |
| Core nextest attempt | Exit 102 before tests because Cargo metadata panicked |

The four expected pre-repair failures expose zero-delay wraparound, wide-integer mismatch, rejection of large admitted attempts, and a deadline-overflow bypass. The exact audit case uses base 2, attempt 63, and maximum delay 128.

The final tests also cover fixed delays, exhausted attempts, invalid policies, missing or invalid jitter, checked jitter-addition overflow, stale generations, and unsupported domains. The tests retain checked deadline overflow and unchanged input values.

## Commands

These commands ran through `nix develop --no-write-lock-file` in the implementation worktree. Build commands used two Cargo jobs and the campaign target directory. Each test or Clippy command had an eight-minute deadline.

```console
cargo test --locked -p molten-core --lib retry_plans_are_bounded_and_jitter_explicit
cargo test --locked -p molten fabric_time
cargo test --locked -p molten-core --lib fabric_time::tests::retry
cargo test --locked -p molten-core --lib
cargo clippy --locked -p molten-core --all-targets --all-features -- -D warnings
cargo nextest run --locked --profile ci -p molten-core
```

The nextest command invoked Cargo metadata with `--all-features --filter-platform x86_64-unknown-linux-gnu --locked`. Cargo panicked in `package_id_spec.rs:248:40`. A default-feature metadata pass does not discharge that failure.

## Remaining acceptance

Shell denial tests, adapter observations, replay divergence, corrected canonical fixtures, strict Octet, broader workspace checks, Nix checks, and Cairn closeout remain required. The current coordination-delivery profile uses fixed backoff. These arithmetic results do not demonstrate a failure through that fixed profile.

No result here proves safe application retries, global liveness, production readiness, or measured performance. No sync, archive, or remote integration follows from this core result alone.
