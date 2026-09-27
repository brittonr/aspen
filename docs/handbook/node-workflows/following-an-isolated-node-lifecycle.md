# Following an isolated node lifecycle

Mode: Walkthrough

This walkthrough follows the checked-in `local_node_init_run_status_stop_and_restart_recovery_are_receipted` regression case, not a hypothetical production deployment. Its input is an isolated process workspace and `node:test`; its outputs are configuration, identity, startup, health, shutdown, and restart evidence. Read the [lifecycle technical companion](../../technical/operations/node-lifecycle-and-recovery.md) for the state model and the [operator runbooks](../../production-operator-runbooks.md) for production review obligations.

Everything below is source-checked, not executed for this document. The regression source is evidence of intended assertions, not a test result from this batch. A successful local sequence would not prove production readiness: no-profile initialization carries `local-fixture-config`, and the inspected startup implementation supplies a synthetic clean Octet gate.

## 1. Establish a genuinely separate workspace

The [regression case](../../../src/node/parts/daemon/tests/m000/p000/body.rs) obtains a process workspace before initialization. Mirror that separation: select an unused node root and a separate unused evidence directory, neither shared with another service nor nested in an existing deployment. Keep permissions suitable for persisted endpoint secrets. Do not initialize over an old root to repair it.

The source path is `src/main.rs` → `cli/ops/node.rs` → the lifecycle shell → `init_local_with_root`. The public shell accepts a host path, opens a `NodeStateRoot`, and passes the acquired capability onward. Descendant operations are not authorized by repeatedly trusting the path string.

The following optional local exercise requires an already-built `molten` on PATH and operator-selected fresh paths. It refuses existing destinations. Command spellings come from the [base declarations](../../../src/cli/ops/node/command/base.rs), [command routing](../../../src/cli/ops/node.rs), and [init shell](../../../src/cli/ops/node/parts/lifecycle/p000/body.rs). Source-checked; not executed.

```sh
: "${NODE_ROOT:?Set an unused isolated node root}"
: "${NODE_EVIDENCE:?Set a separate unused evidence directory}"
test ! -e "$NODE_ROOT" && test ! -e "$NODE_EVIDENCE" &&
mkdir -p "$NODE_EVIDENCE" &&
molten node init --state-root "$NODE_ROOT" --node-id node:test \
  --config-out "$NODE_EVIDENCE/config.preserves" \
  --identity-receipt-out "$NODE_EVIDENCE/identity-receipt.preserves" \
  --profile-resolution-out "$NODE_EVIDENCE/profile-resolution.preserves"
```

## 2. Identify what initialization actually publishes

`init_local_with_root` checks lifecycle emptiness, creates the layout, resolves endpoint identity through the identity namespace, and constructs local profile evidence. It writes `config.preserves`, `profile-resolution.preserves`, `identity-receipt.preserves`, and `identity.preserves` at the node root. The private persisted key belongs under `identity/`; it is not an evidence export.

The regression checks canonical configuration and profile-resolution refs, then confirms the local-fixture caveat. Preserve that caveat in your notes. The node identifier is an input label, not proof of peer admission or authorization.

## 3. Cross the startup boundary

The next source call is `run_local`. It checks restart state, reads configuration and identity evidence, obtains runtime startup receipts, writes adapter-start receipts, and writes the startup receipt. Only a passing decision proceeds to the startup-bound active lock and startup ledger import. These writes are ordered effects, not one atomic commit.

For an optional exercise, continue only after reviewing successful initialization. These commands use the [same declarations](../../../src/cli/ops/node/command/base.rs) and the [run/status/stop shells](../../../src/cli/ops/node/lifecycle.rs); their effect ordering is in [daemon startup and shutdown](../../../src/node/parts/daemon/p018/body.rs). Source-checked; not executed.

```sh
: "${NODE_ROOT:?Use the initialized isolated root}"
: "${NODE_EVIDENCE:?Use its separate evidence directory}"
molten node run --state-root "$NODE_ROOT" \
  --startup-out "$NODE_EVIDENCE/startup.preserves" &&
molten node status --state-root "$NODE_ROOT" \
  --health-out "$NODE_EVIDENCE/running-health.preserves" \
  --receipt-out "$NODE_EVIDENCE/status-control.preserves" &&
molten node stop --state-root "$NODE_ROOT" \
  --shutdown-out "$NODE_EVIDENCE/shutdown.preserves" \
  --receipt-out "$NODE_EVIDENCE/stop-control.preserves"
```

Here `run` performs local lifecycle startup and returns; it is not the bounded control loop or live listener. Do not infer a continuously serving network process from its name.

## 4. Interpret status and shutdown separately

The test expects status `running` before stop and `stopped` afterward. Status derives that projection from startup and shutdown artifacts; it also writes health and control evidence and imports them. It is not a read-only process probe.

Stop first admits a shutdown plan. Denial writes denial evidence without executing that plan. An admitted stop publishes adapter shutdown, node shutdown, and control receipts before removing the active lock. Preserve the exported pre-restart shutdown artifact: it ties the stopped observation to the prior startup.

## 5. Follow the restart and refusal branches

The regression restarts after a clean stop, then deliberately attempts another run without another stop. That final attempt must fail because the previous startup has no clean shutdown receipt. Do not reproduce this negative case in a shared deployment.

On the clean path, restart writes restart-health evidence and requires a passing decision before removing the old root shutdown file. Later status can overwrite the root health file, so a single filename is not a history. On any uncertain failure, retain the root and exported artifacts; do not delete the lock or manufacture shutdown evidence.

## Sources

- [Handbook](../README.md)
- [Lifecycle technical companion](../../technical/operations/node-lifecycle-and-recovery.md)
- [Production operator runbooks](../../production-operator-runbooks.md)
- [Node-state authority contract](../../node-state-filesystem-authority.md)
- [Concrete lifecycle regression](../../../src/node/parts/daemon/tests/m000/p000/body.rs)
- [Initialization implementation](../../../src/node/parts/daemon/p045/body.rs)
- [Restart implementation](../../../src/node/parts/daemon/p036/body.rs)
