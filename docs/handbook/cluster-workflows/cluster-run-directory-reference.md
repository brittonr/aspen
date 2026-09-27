# Cluster run directory reference

Mode: Reference

This is a lookup guide for a receipt-first local cluster export, not a schema replacement. Names and fields below are source-checked; no run was executed for this article. Canonical Preserves values and BLAKE3 references define artifact identity, not Rust struct layout or a filename. See the [technical companion](../../technical/operations/receipt-first-cluster-diagnosis.md) for the underlying evidence model.

## Directory-level artifacts and owners

The [runner constants](../../../src/cluster_harness/parts/runner/p000/body.rs) own filenames. The [canonical builders](../../../src/cluster_harness/parts/canonical/p000/body.rs) and [parent/verification builders](../../../src/cluster_harness/parts/canonical/p001/body.rs) own the corresponding record construction.

| Relative path | Indexed kind or role | Producer and review use |
| --- | --- | --- |
| `artifact-index.tsv` | Index, not an indexed child | Runner: binds paths, kinds, refs, formats |
| `fixture-metadata.preserves` | `cluster-harness-fixture-metadata` | Planner: fixture ref, source kind, ordered nodes, caveats |
| `command-plan.preserves` | `cluster-harness-command-plan` | Planner: phases, node order, timeout, expected kinds |
| `derived-plan.preserves` | `local-multiprocess-plan` | Local multiprocess builder: state and transport handles |
| `local-executable-run.preserves` | `local-multiprocess-executable-run` | Run recorder: shell observations bound to plan |
| `cluster-lifecycle-receipt.preserves` | `cluster-lifecycle-run` | Lifecycle builder: phase and per-node evidence |
| `drift-summary.preserves` | `cluster-harness-drift-summary` | Summary builder: receipt-derived comparison fields |
| `cleanup-receipt.preserves` | `cluster-harness-cleanup` | Cleanup builder: process, stop, orphan, and ticket observations |
| `cluster-run-receipt.preserves` | `cluster-harness-run` | Parent builder: binds the overall evidence set |
| `verification.preserves` | Unindexed verification companion | Finalizer: original offline assessment |
| `failure-repro-bundle.preserves` | Optional unindexed failure companion | Denied-run finalizer: sealed diagnostic bundle |
| `failure-repro-verification.preserves` | Optional unindexed failure companion | Failure-bundle verifier: separate diagnostic verification |

The eight required indexed kinds are fixture metadata, command plan, local plan, local executable run, lifecycle, drift summary, cleanup, and parent. That required-kind list is owned by the [pure core assessment](../../../crates/molten-core/src/cluster_harness.rs). It is a directory-level minimum, not a declaration that eight files alone prove a complete two-node execution.

## Index record format

The literal first line is `molten.cluster-run-index.v1`. Subsequent nonempty rows have four tab-separated columns, in this order:

| Column | Meaning | Checks to distinguish |
| --- | --- | --- |
| Relative path | Artifact location inside the run | Safe relative spelling, unique path, sorted order |
| Artifact kind | Expected semantic kind | Nonempty unpadded kind; observed kind must match |
| Expected ref | Content identity | Valid BLAKE3 reference and observed equality |
| Format | `preserves` or `text` | Supported format and observation agreement |

The core limit is 4096 indexed artifacts. The shell parser permits one extra entry so the core can emit its oversized-index diagnostic; still larger inputs can fail during parsing. Canonical Preserves artifacts must also match canonical text rendering. Text refs use a domain-separated BLAKE3 computation, so a generic file checksum is not interchangeable with the indexed text ref.

## Child evidence paths

These are path patterns, not executable commands. `NODE` means the planner's safe path component, such as `fixture-a`; `PHASE` is one of `init`, `start`, `workflow`, `status`, or `stop`.

| Path pattern | Meaning and owner |
| --- | --- |
| `children/processes/PHASE-NODE.preserves` | Runner's `cluster-harness-child-process` observation |
| `logs/PHASE-NODE.log` | Runner's indexed `cluster-harness-diagnostic-log`, format `text` |
| `children/receipts/NODE/config.preserves` | Captured node configuration |
| `children/receipts/NODE/identity-receipt.preserves` | Captured node identity evidence |
| `children/receipts/NODE/startup-receipt.preserves` | Captured startup evidence |
| `children/receipts/NODE/cluster-harness-workflow.preserves` | Bounded workflow output |
| `children/receipts/NODE/cluster-harness-heartbeat.preserves` | Workflow heartbeat output |
| `children/receipts/NODE/health-receipt.preserves` | Status health evidence |
| `children/receipts/NODE/status-control-receipt.preserves` | Status control evidence |
| `children/receipts/NODE/shutdown-receipt.preserves` | Shutdown evidence |
| `children/receipts/NODE/stop-control-receipt.preserves` | Stop control evidence |

The capture function discovers each node artifact's actual kind rather than deriving it from its filename. Missing source files are skipped during capture; completeness is evaluated afterward. Attempted process phases have their own observations, distinct from the node artifacts they were expected to produce.

## Fields useful during review

The parent record exposes `decision`, `fixture`, `command-plan`, `local-plan`, `local-run`, `lifecycle`, `drift-summary`, `cleanup`, `child-receipts`, `diagnostic-logs`, required/observed artifact kinds, diagnostics, caveats, and checks. The child-process record exposes `node`, `phase`, `command-profile`, `diagnostic-log`, `exit-code`, `timed-out`, and `orphaned`, alongside its decision and checks.

Verification binds `artifact-index`, diagnostics, and `first-divergence`. A divergence contains `path`, `artifact-kind`, `expected`, `observed`, `reason`, and diagnostic-only scope. This identifies an assessment mismatch, not necessarily the causal first failure in process time.

Drift summaries contain fields plus explicit expected equalities and allowed variances. Their diagnostic-only scope does not authorize arbitrary differences. In particular, index integrity still requires every observed artifact to match its own expected ref.

## Worked lookup and limits

For fixture-a's startup, inspect the index row for `children/receipts/fixture-a/startup-receipt.preserves`, then the start process receipt and its bound log. If the startup file exists but differs from its index ref, a successful child exit cannot repair that mismatch. If it is absent, the start process observation helps distinguish spawn failure from a child that exited without the required output.

Only the index, verification companion, and two named failure companions are exempt from unexpected-file detection. Exemption does not validate a failure bundle's payload. Offline verification compares the existing verification companion when the indexed assessment passes; it does not refresh that companion. Receipts describe this local evidence set and grant neither current authority nor VM/live/production readiness.

## Sources

- [Handbook](../README.md)
- [Governing harness guide](../../receipt-first-cluster-harness.md)
- [Technical diagnosis companion](../../technical/operations/receipt-first-cluster-diagnosis.md)
- [Filename and runner input definitions](../../../src/cluster_harness/parts/runner/p000/body.rs)
- [Node artifact capture](../../../src/cluster_harness/parts/runner/p003/body.rs)
- [Index serialization and observations](../../../src/cluster_harness/parts/runner/p004/body.rs)
- [Core required kinds and assessment](../../../crates/molten-core/src/cluster_harness.rs)
