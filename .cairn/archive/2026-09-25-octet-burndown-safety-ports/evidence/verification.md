# Verification: Octet burn-down, bounded channels and fixed-width public integers

Base: `integration/stack-20260925` at `9602f924c`. That is `cde316081` plus the fabric-boundary compatibility fixtures
and the fabric_execution stdin (EPIPE) fix. Octet: pinned `octet-toolchain` `fc38f593`. Private `CARGO_TARGET_DIR`;
`nice -n 10`, `CARGO_BUILD_JOBS=16`, Nix `--max-jobs 2`.

## Octet

| Scope | Base findings | After | `unbounded_channel` findings (sites) | `usize_in_public_api` findings (sites) |
|---|---:|---:|---:|---:|
| workspace (`cargo octet check`) | 3250 | 3209 | 9 (9) → 0 | 32 (16) → 0 |
| `-p molten --lib` | 1379 | 1363 | 0 → 0 | 16 (16) → 0 |

`octet-lint-diff.txt` shows that no other family changed. The first candidate run added one `path_segment_repetition`
finding: the new test name repeated `port` from `port_tests`. The test was renamed
(`full_supervision_queue_denies_publication_without_enqueueing`), and the rerun is level.

## Channels

- The inbox capacity is the admitted time profile's `max_scheduler_queue_depth`, converted with `usize::try_from`. A
  zero or unconvertible depth denies setup.
- The control capacity is `LIVE_CONTROL_OBSERVATION_CAPACITY` = 32. The bounded live workflows publish at most 6
  observations and never drain the channel.
- Timer tasks `send(..).await`: a full inbox parks the timer task and delivers the event after the receiver drains.
- `ChannelReplicaControlPort::publish` uses `try_send`. `Full` returns "live Raft supervision queue is full" and
  enqueues nothing. `Closed` keeps "receiver is unavailable".
- Deadlock analysis: timer sends run in their own spawned tasks and hold no lock. The service owns the receiver and
  never sends to its own inbox. The control port never blocks.
- Tests:
  - `full_supervision_queue_denies_publication_without_enqueueing`: exactly `capacity` observations are accepted. One
    more is denied, the queue length stays `capacity`, and the queued receipts are unchanged.
  - `full_event_queue_parks_timer_delivery_until_drained`: a capacity-1 inbox with two armed timers fills and stays at
    capacity, then delivers both events after draining. This shows there is no deadlock under a full queue.
  - The existing closed-receiver and zero-generation tests are adapted.

## Fixed-width public integers

Public signatures now use `u64`/`u32`. `crate::bounded::{usize_from_u64,u64_from_usize}` return a labelled
`MoltenError` when a value does not fit, and no `as` cast is added. Crate-internal callers that already hold a `usize`
policy bound use the crate-private `MaterializationPath::parse_within`. Record-arity constants used only as arities are
`u64`. `canonical_world_sync_receipt` encodes `verified` with `u64_value`, the same Preserves integer the previous
`usize_value` produced.

## Rust gates

- `cargo fmt --check`: exit 0. `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo test --workspace --no-fail-fast`: exit 0, 2091 passed, 0 failed, 0 ignored. That includes
  `fabricboundarycompat`, 5 of 5 passed.
- The diff adds no `allow`. `dylint.toml`, baselines, and quarantine files are unchanged.

## Byte identity against the base binary (`9602f924c`)

- `fixture-receipt-comparison.txt`: all 6 `examples/*.preserves` harness suites give identical exit codes and report
  and gate-receipt BLAKE3 hashes.
- `simulation-fixture-comparison.txt`: `molten fabric-simulation run` (100 artifacts) and `shrink` (3) are identical.

## Review and lifecycle

The no-spec plan review approves the proposal, design, acceptance, and tasks with no findings. The proposal, design,
and tasks gates pass, and `cairn validate --strict` reports no issues.
