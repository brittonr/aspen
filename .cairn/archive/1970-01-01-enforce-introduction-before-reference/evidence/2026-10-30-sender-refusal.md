# Enforce introduction before reference — sender-side refusal

The remaining task is satisfied by the separately governed `recorded-remote-session-source`
prerequisite. Sender builds consume its live receiver-owned view; an unintroduced semantic
reference refuses with canonical evidence before any envelope exists, and local publication
rechecks the exact source.

## Executed evidence

- `meshspace::tests::*` introduction fixtures (bootstrap, assertion, retraction, unknown reference,
  closed session) and `meshspace::recorded_session_tests::*` pass.
- `cli_recorded_remote_session_refuses_unknown_and_revoked_references` exercises sender refusal
  through the real CLI build, publish-local and receive paths.

## Integrated source checks (executed 2026-10-30)

Run in the integration worktree `cairn/finish-open-changes-20261029` (base
`d7e435e711f7f58167900ce7419121390b0842d3`) through `nix develop` (`nightly-2026-05-26`):

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p molten --all-targets --features spacewasm-pilot -- -D warnings` | exit 0 |
| `cargo clippy -p molten --all-targets --features component-rpc-telemetry -- -D warnings` | exit 0 |
| `cargo octet check --artifact-dir …` | `Status: clean`, 0 findings |
| `cargo octet check -p molten --artifact-dir … -- --lib` | `Status: clean`, 0 findings |
| `cargo test --workspace --no-fail-fast` | 2,907 passed, 0 failed, 28 ignored |
| `nix build .#checks.x86_64-linux.inherited-tracey-debt` | pass, `dangling=0` |
| `cairn validate --root . --policy ../cairn/cairn-policy/generated/cairn-policy.json --strict` | `valid: true` |

The pinned Octet producer used by the startup source binding (cargo-octet 0.1.0 on
`nightly-2026-03-21`, Nix cohort) also reported `Status: clean` for the primary and library
cohorts. The 28 ignored tests require independently provisioned signed producer outputs or
external cohorts and are reported only where they were explicitly executed.
