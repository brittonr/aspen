# Verification: node shutdown admission (F01)

## Baseline

The pre-change baseline ran on the integration checkout
`/home/brittonr/git/OnixResearch/aspen` at commit `38cb87acd` (branch `molten`) with no
F01 source edit present:

- `cargo test -p molten --lib -- shutdown`: 8 passed (existing shutdown dispatch,
  daemon-loop, and supervisor controls).
- `cargo test -p molten --lib -- local_node_init_run_status_stop_and_restart_recovery_are_receipted`:
  1 passed.

## Executed reproduction

`daemon_core::tests::shutdown_dispatch_without_authority_denies_before_effects` from this
change's `src/node/parts/daemon/tests/m000/p014/body.rs` was compiled against the pre-change
shell: only the test file and its `p038` include were added, with no implementation edit.
It failed:

```text
running 1 test
test daemon_core::tests::shutdown_dispatch_without_authority_denies_before_effects ... FAILED

thread 'daemon_core::tests::shutdown_dispatch_without_authority_denies_before_effects' panicked at
.../src/node/parts/daemon/tests/m000/p014/body.rs:58:9:
denied shutdown preserves the active lock

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1462 filtered out
```

The failing assertion is the F01 symptom: pre-change dispatch removed the active lock (and
published adapter shutdown receipts) while the control receipt reported `deny`. The same
test passes with the change applied, in both the dispatch and direct-stop paths.

## Implementation evidence

- `src/node/parts/runtime/p005/body.rs`: `admit_node_shutdown` is a pure decision over
  `ControlRequest`, startup receipt ref, startup adapter receipts, and active-lock facts.
  It performs no filesystem access, writes no receipts, bounds adapter and diagnostic
  counts, and returns either diagnostics with no plan or a typed `ShutdownPlan` whose
  adapter order is the validated reverse start order.
- `src/node/parts/daemon/p018/body.rs`: direct stop reads the current startup receipt,
  calls admission before any adapter or shutdown write, publishes diagnostic denial
  evidence on rejection, and executes the admitted plan only on `pass`. An admitted plan
  whose effect fails returns the shell error, leaves the active lock in place, and publishes
  no successful shutdown or control-stop receipt.
- `src/node/parts/daemon/p027/body.rs`: queued shutdown dispatch runs the same admission
  before protected shell effects and routes denial through `finalize_operation_dispatch`
  with `subreceipt_refs=[]` and the admission diagnostics.
- `src/node/parts/runtime/tests/m000/p002/body.rs` and
  `src/node/parts/daemon/tests/m000/p014/body.rs` carry the kernel and daemon regressions.

## Checks

Executed in `/home/brittonr/git/OnixResearch/aspen-w3`:

- `cargo fmt --all` then `cargo fmt --check`: clean.
- `cargo check --workspace --all-targets`: finished with no errors.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p molten --lib`: 1466 passed.
- `cargo test -p molten --lib -- shutdown`: 15 passed.
- `cargo test -p molten --lib -- direct_stop`: 1 passed.
- `cargo octet check --artifact-dir target/octet-shutdown-admission-3` (workspace scope):
  3619 findings, 0 errors, `warning-only`. The pre-change checkout at `38cb87acd` reports
  3619 findings with the same tool and toolchain, so this change adds no finding. Earlier
  revisions of this change did add `function_length` and `borrowed_argument_types`
  findings; the final revision splits admission into short helpers and takes
  `&mut impl crate::bounded::VecSink<String>` sinks to return to the baseline.
- Cairn: `validate --root . --strict` valid; `gate proposal|design|tasks` valid for this change.

Not run: Nix node-state and flake checks (see caveats).

## Traceability

- `r[molten.audit_f01.admission]`: `admit_node_shutdown` plus
  `stop_local_node_with_request` and `dispatch_shutdown_request` route admission before
  protected effects; kernel and daemon tests cover pass and deny decisions.
- `r[molten.audit_f01.preserve_state]`: denial comparisons in
  `shutdown_dispatch_without_authority_denies_before_effects`,
  `shutdown_denials_preserve_lifecycle_state_across_rejection_cases`, and
  `direct_stop_without_active_lock_denies_and_preserves_state` compare startup bytes,
  active-lock presence, shutdown-receipt presence, and adapter-shutdown receipt counts
  before and after denial.
- `r[molten.audit_f01.observed_effects]`: `execute_shutdown_plan` publishes receipts only
  after running the admitted plan; `admitted_shutdown_closes_adapters_in_reverse_start_order`
  checks observed adapter order and `shutdown_effect_error_cannot_become_success` checks a
  failing effect cannot publish success.
- `r[molten.audit_f01.validation]`: executed results are separated from the static audit
  evidence in this file, and the reproduction above records the pre-change failure.

## Receipt and replay compatibility (F14 interface review)

- Denial publishes `deny` control/operation receipts with admission diagnostics and an
  empty subreceipt set. It never writes `SHUTDOWN_FILE`, never writes adapter shutdown
  receipts, and never removes the active lock, so a denied request cannot be replayed as an
  admitted effect plan or as a current stopped observation.
- A prior successful `SHUTDOWN_FILE` stays byte-identical after a later denied stop, which
  is the input condition `bind-shutdown-observations-to-current-node-run` (F14) needs to
  separate historical shutdown success from current run state. F14 remains an open package
  and owns that observation change; this package does not reinterpret historical receipts.

## Caveats

- Nix checks were not run for this change; focused Cargo gates cover the changed surface and
  no dependency, lockfile, or flake input changed.
- The reproduction is a public-path test run of the pre-change shell, not a live adapter
  shutdown or a crash-atomicity test.

## Claim boundary

The evidence shows that shutdown admission now decides over supplied typed facts, that
denied requests leave lifecycle artifacts unchanged in the covered paths, and that admitted
plans publish receipts only from observed work. It does not prove remote authentication,
complete adapter shutdown, crash atomicity, live-node correctness, or release readiness.
