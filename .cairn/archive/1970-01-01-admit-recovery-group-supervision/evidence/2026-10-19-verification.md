# Recovery-group supervision verification

## Baseline

`plan_supervision` made one decision per failure from the failure class and a
lifetime restart count, and `LifecycleState` carried no group membership or
time-windowed history. The manifest had no recovery-group declaration.

## Implementation

- `RecoveryGroupDeclaration` admits an ordered member list with per-member
  restart class (`permanent`, `transient`, `temporary`), ordering dependencies,
  an elapsed-time window and source ref, backoff, member budget, and group
  budget.
- `validate_recovery_group` rejects empty or oversized groups, malformed ids,
  duplicate members and dependencies, self dependencies, unknown dependencies,
  ordering conflicts (including cycles), invalid windows/backoffs/budgets, and
  missing or malformed elapsed-source refs.
- `SystemExtensionManifestInput` and the admitted manifest carry an optional
  recovery group. Admission reports issues as typed manifest issues.
- `plan_recovery` is a pure planner over the admitted group, the failing
  member's exit, supplied bounded restart history, and the current elapsed
  tick. It returns an ordered stop/restart/quarantine/escalate plan.
- Permanent members restart within budget; transient members stop on success
  and restart only abnormal retryable exits; temporary members stop on success.
  The planner quarantines on member budget or backoff exhaustion and escalates
  on group budget exhaustion. A one-member permanent group matches
  `plan_supervision` for retryable and non-retryable failures.
- History is supplied by the caller, so replacing a supervisor does not reset
  the windowed member or group budget. Planning is deterministic and consumes
  only admitted elapsed observations.

## Validation

- `cargo test -p molten-core --lib system_extension`: 39 passed, 0 failed.
- `cargo test -p molten --lib fabric_time`: 37 passed, 0 failed.
- `cargo test -p molten-core --lib`: 402 passed, 0 failed.
- `cargo test -p molten --lib`: 1,572 passed, 0 failed.
- `cargo check --workspace --all-targets`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo fmt --all --check`: pass.
- `cargo octet check --artifact-dir target/octet-recovery-group-3`: clean,
  0 findings, 0 warnings, 0 errors.
- Admission tests cover valid, duplicate, unknown, self, cyclic, and invalid
  bound/source shapes. Planning tests cover ordered suffix restart, independent
  members, restart classes, member/group budgets, backoff, expired-window
  history, invalid history, and deterministic replay.

## Non-claims

The plan is advisory until the shell executes it; it does not prove restart
success, failure detection, arbitrary dependency-graph solving, or real-time
behavior beyond admitted elapsed observations. Redelivery and external-effect
retry remain separate mechanisms. No whole-system or release claim is made.
