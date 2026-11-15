# Command families and artifact reference

Mode: Reference

This is a routing and artifact-ownership desk reference, not a replacement for leaf declarations or a claim that every listed family is production-ready. Spellings and behaviors are source-checked; no commands or tests were executed for this article. The [Handbook](../README.md) provides task-oriented routes. For planning semantics, consult the [technical companion](../../technical/world-effects/preview-first-operator-composition.md).

## Root navigation

The executable parser is named `molten`. [main.rs](../../../src/main.rs) maps modules with `#[path]` and re-exports readable `cli_*` aliases. [root.rs](../../../src/main/root.rs) parses and dispatches; [command.rs](../../../src/main/root/command.rs) includes declaration part `p000` and parser-test part `p001`. A filesystem search for the alias name alone can miss the implementation.

These rows are navigation keys, not runnable recipes. Family presence proves a parser route, not the behavior of every nested verb.

| Root family or group | CLI owner resolved by `main.rs` | Routing purpose |
| --- | --- | --- |
| `cluster`, `dogfood`, `receipts` | `cli/ops/cluster/mod.rs`, `cli/ops/dogfood/mod.rs`, `cli/evidence/receipts.rs` | Cluster, operational rails, and receipt surfaces |
| `node`, `peer` | `cli/ops/node.rs`, `cli/ops/peer.rs` | Node and peer operations |
| `runtime config` | `main/root.rs` | Read an exported startup configuration and summarize it |
| `fabric-time` | `cli/runtime/fabric_time/mod.rs` | Executable time fixture and report readback |
| `fabric-simulation`, `system-extension` | Corresponding modules under `cli/runtime/` | Simulation and extension surfaces |
| `world` | `cli/runtime/world.rs` | Request planning and fail-closed standalone apply |
| `world-commit`, `world-snapshot`, `world-authority`, `world-head` | `worldcommit.rs`, `worldsnapshot.rs`, `worldauthority/mod.rs`, `worldhead.rs` under `cli/runtime/` | Component-owned world surfaces |
| `world-distribution`, `world-merge`, `world-promotion` | `worlddistribution.rs`, `worldmerge.rs`, `worldpromotion.rs` under `cli/runtime/` | Distribution, comparison, and promotion surfaces |
| `test` | `run_test_command` in `main/root.rs` | Nested harness, artifact, workflow, runtime, and evidence routes |

With no subcommand, the root dispatcher prints a greeting. That is not a daemon startup or configuration validation.

## Frequently confused nested routes

The [test enum](../../../src/main/root/parts/command/p000/body.rs) declares positional suite input for `test run` and positional report input for `test replay`. Their optional destinations are `--report-out` and `--failure-out`, respectively. The same enum also nests `report`, `gate`, `receipt`, `ledger`, `chain`, `chunk`, `storage`, `artifact`, `schema`, `cache`, `transcript`, `catalog`, and additional workflow/runtime families. Their specific leaf contracts are independent; an output option on one is not inherited by another.

`node` and `test node` dispatch to the same implementation. `receipts` at the root and `test receipt` use distinct command types from the receipts module. `test raft` remains a control-plane-oriented family; its presence does not imply OpenRaft, general actor-message consensus, or exactly-once delivery.

## Inspected leaf argument shapes

Provenance for the table is [WorldCommand and argument structs](../../../src/cli/runtime/world.rs), [ClockAction](../../../src/cli/runtime/fabric_time/command.rs), and [root Runtime/Test declarations](../../../src/main/root/parts/command/p000/body.rs).

| Route | Required inputs/destinations | Optional or bounded selections |
| --- | --- | --- |
| `world plan` | `--request`, `--out` | `--receipt-out`, `--summary-out` |
| World single-operation read variants | `--request`, `--plan-out` | `--receipt-out`, `--summary-out` |
| World mutation variants | `--request`, `--plan-out` | `--summary-out`, `--apply-plan-ref`, `--receipt-out`; apply requires receipt output at runtime |
| `fabric-time run-fixture` | No required flag | `--profile`: `live`, `deterministic-simulation`, `both`; `--out` selects a directory |
| `fabric-time show` | Positional report path | Reader enforces a 1,048,576-byte metadata limit |
| `runtime config` | `--config` | Handler reads Nickel-export JSON, not source Nickel evaluation |

World read variants include inspect, diff, conflicts, replay, simulate, verify, export, and gc-plan. Mutation variants are checkpoint, branch, run, promote, and import. These classifications come from the CLI enum and routing, not ordinary-language guesses about the verbs.

## Artifact owners and representations

| Artifact | Producer/consumer | Meaning and important limit |
| --- | --- | --- |
| World JSON request | `world/document.rs` | Explicit facts; unknown fields denied; not a live authority grant |
| World plan destination | `world/output.rs` | Canonical plan bytes; not proof of component execution |
| World receipt destination | Same writer and denial writer | Planning evidence or denial; apply can replace the initially written receipt |
| World summary destination | Same writer | Canonical summary bytes, separate from terminal rendering |
| `profiles/live.preserves` and `profiles/deterministic-simulation.preserves` | Fabric-time artifact planner | Both profile records are included in the output plan |
| `report.preserves` | Fabric-time fixture/readback | Preserves text encoding of the canonical run value |
| `evidence/` numbered event files | Fabric-time artifact planner | Individual event values, not acceptable substitutes for the run report |

The world writer writes canonical record bytes; the time writer renders values using `to_text`. Therefore `.preserves` does not alone tell a generic reader which representation to expect. Canonical identity is defined by Preserves and BLAKE3, never by pathname or Rust DTO layout.

## Worked lookup

Suppose an operator has a time event and wants a summary. The table identifies `fabric-time show` as a run-report consumer, so supplying that event is a type-boundary error, not evidence of corrupt time execution. Locate the enclosing `report.preserves` instead. Likewise, the logical world request contains a graph: route it to `world plan`, not `world inspect`. The [retained request tests](../../../src/cli/runtime/world/tests.rs) establish the intended graph-to-record comparison, without authorizing live application.

## Sources

- [Handbook](../README.md)
- [Preview-first operator composition](../../technical/world-effects/preview-first-operator-composition.md)
- [World operator contract](../../world-operator-workflows.md)
- [Entrypoint aliases](../../../src/main.rs)
- [Root declarations](../../../src/main/root/parts/command/p000/body.rs)
- [World output writer](../../../src/cli/runtime/world/output.rs)
- [Fabric-time artifact planner and reader](../../../src/cli/runtime/fabric_time/ops.rs)


## Retained command examples

The detailed examples formerly in the root README are retained here, not
newly executed recipes. **Run every command from the repository root**, not
this document's directory. Relative `target/`, `examples/`, `tests/`, `src/`
and sibling input paths retain their repository-root meaning. `nix develop`
supplies tools, not an installed `molten`; use `cargo run -- …` in place of
`molten` unless you selected an exact built binary. Allocate fresh task-owned
state and output roots before adapting these examples; do not pre-delete or
force-reinitialize existing user state. `blake3:…` and illustrative refs are
argument-shape examples, not admitted evidence. Cluster and live workflows
require independently provisioned source trust and operation authority.
Source-defined behavior below is not current execution qualification; consult
the [exact capability report](../../capability-qualification-report.md).

## Standalone batch orchestration pilot

The separate seven-crate workspace in `pilots/orchestration` runs bounded,
import-free component DAGs through real Redb, Iroh and Wasmtime adapters. It is not
part of the root Cargo workspace or an admitted Molten system extension. See
[`docs/standalone-batch-orchestration.md`](../../standalone-batch-orchestration.md)
for provenance, strict verification, the real-process demo and gated integration
boundaries.

## Schema identity core pilot

Molten runs a bounded adapter pilot against the published `schema-identity-core` cohort. See [`docs/schema-identity-core-pilot.md`](../../schema-identity-core-pilot.md).

## Node runtime daemon

The cluster convenience wrapper initializes and controls per-node state roots while preserving the same node receipts. For source-tree development, run it through Cargo or an already-built Cargo binary; `nix develop` provides tools but does not install a freshly built `molten` binary by itself. Re-running `cluster init` over an existing cluster manifest or node lifecycle state fails closed; pass `--force` only when you intentionally want to remove the planned node roots and write fresh lifecycle evidence.

```sh
cargo run -- cluster init --state-root target/cluster --node node-a --node node-b
cargo run -- cluster init --state-root target/cluster --node node-a --node node-b --force
cargo run -- cluster start --state-root target/cluster
cargo run -- cluster status --state-root target/cluster
cargo run -- cluster stop --state-root target/cluster
```

For CI and operator review, the receipt-first wrapper runs a checked cluster fixture through independent node processes and writes one content-addressed run directory with child receipts, a canonical parent receipt, drift/cleanup evidence, and an offline verification receipt. Diagnostic logs remain adjuncts and cannot replace canonical evidence. See [`docs/receipt-first-cluster-harness.md`](../../receipt-first-cluster-harness.md).

```sh
cargo run -- cluster harness-run --fixture tests/fixtures/cluster-harness/two-node.cluster --state-root target/cluster-harness-state --run-dir target/cluster-harness-run --node-binary "$SIGNED_OUTPUT/bin/molten" --source-trust "$SOURCE_TRUST"
cargo run -- cluster harness-verify --run-dir target/cluster-harness-run
```

The Nix-built flake app is available when the private sibling inputs are supplied explicitly:

```sh
nix run \
  --override-input basalt-src path:../basalt \
  --override-input cairn-src path:../cairn \
  --override-input valence-src path:../valence \
  --override-input onixpkgs path:../onixpkgs \
  .#molten -- cluster status --state-root target/cluster
```

The durable local node boundary is exposed at top level under `molten node`:

```sh
molten node init --state-root target/node --node-id node:local
molten node run --state-root target/node
molten node control-request --operation status --out target/node.status-request.preserves \
  --authority blake3:authority --policy blake3:policy --resource blake3:resource
molten node provenance-fixture --artifact-ref blake3:payload --out target/node.payload-provenance.preserves
molten node authority-grant-fixture --state-root target/node --peer peer:operator --node node:local \
  --operation status --policy blake3:policy --out target/node.operator-grant.preserves
molten node live-ticket-export --state-root target/node --policy blake3:policy \
  --out target/node.live-ticket.preserves
molten node live-peer-admit --state-root target/node --peer peer:operator --policy blake3:policy \
  --receipt-out target/node.operator-peer-admission.preserves target/node.live-ticket.preserves
molten node live-ticket-import --state-root target/sender-node target/node.live-ticket.preserves \
  --peer-admission target/node.operator-peer-admission.preserves \
  --expected-node node:local --expected-topic node-control --expected-peer peer:operator \
  --receipt-out target/sender-node.live-ticket-import.preserves
molten node authority-grant-import --state-root target/sender-node target/node.operator-grant.preserves \
  --peer peer:operator --node node:local --operation status \
  --receipt-out target/sender-node.authority-grant-import.preserves
molten node supervisor-policy-fixture --state-root target/node --max-restarts 1 \
  --restart-window-ticks 8 --heartbeat-timeout-ticks 4 --shutdown-drain-ticks 2 \
  --allow-stale-lock-recovery --policy blake3:policy \
  --out target/node.supervisor-policy.preserves
molten node control-submit --state-root target/node target/node.status-request.preserves \
  --receipt-out target/node.status-queue.preserves
molten node control-ingress-build target/node.status-request.preserves \
  --from-peer peer:operator --to-node node:local --peer-bootstrap blake3:bootstrap \
  --authority blake3:authority --policy blake3:policy --resource blake3:resource \
  --out target/node.status-ingress.preserves
molten node control-ingress-publish --state-root target/node target/node.status-ingress.preserves \
  --receipt-out target/node.status-ingress-publish.preserves
molten node control-ingress-deliver --state-root target/node blake3:ingress-envelope \
  --receipt-out target/node.status-ingress-deliver.preserves
molten node control-ingress-live-loopback --state-root target/node target/node.status-request.preserves \
  --from-peer peer:operator --to-node node:local --peer-bootstrap blake3:peer-admission \
  --authority blake3:operator-grant --policy blake3:policy --resource blake3:resource \
  --receive-receipt-out target/node.status-live-receive.preserves
molten node control-ingress-live-send --state-root target/sender-node \
  target/node.status-request.preserves target/node.bound-live-ticket.preserves \
  --from-peer peer:operator --operation-id blake3:expected-operation \
  --expected-node node:local --expected-topic node-control --max-attempts 2 \
  --peer-bootstrap blake3:peer-admission \
  --authority blake3:operator-grant --policy blake3:policy --resource blake3:resource \
  --transport-receipt-out target/node.status-live-send-transport.preserves \
  --retry-receipts-dir target/node.live-send-retries \
  --duplicate-receipt-out target/node.status-live-duplicate.preserves \
  --receipt-out target/node.status-live-send.preserves
molten node run-loop --state-root target/node --max-requests 8 \
  --receipt-out target/node.control-loop.preserves
molten node serve --state-root target/node --max-ticks 64 --max-requests-per-tick 8 \
  --supervisor-policy target/node.supervisor-policy.preserves \
  --receipt-out target/node.service-run.preserves
molten node serve --state-root target/node --live-iroh --live-max-events 16 \
  --service-receipt-out target/node.service-run.preserves \
  --live-ticket-out target/node.bound-live-ticket.preserves \
  --receipt-out target/node.live-listener.preserves
molten node live-workflow-bundle-export \
  --ticket target/node.bound-live-ticket.preserves \
  --peer-admission target/node.operator-peer-admission.preserves \
  --authority-grant target/node.operator-grant.preserves \
  --receipt target/node.status-live-send.preserves \
  --out target/node.live-workflow-bundle.preserves \
  --receipt-out target/node.live-workflow-bundle-export.preserves
molten node live-workflow-bundle-verify target/node.live-workflow-bundle.preserves \
  --expected-node node:local --expected-topic node-control --expected-peer peer:operator \
  --operation status --receipt-out target/node.live-workflow-bundle-verify.preserves
molten node live-workflow-bundle-gate target/node.live-workflow-bundle.preserves \
  --verify-receipt target/node.live-workflow-bundle-verify.preserves --require-verify-receipt \
  --expected-node node:local --expected-topic node-control --expected-peer peer:operator \
  --operation status --receipt-out target/node.live-workflow-bundle-gate.preserves
molten node live-workflow-bundle-apply --state-root target/sender-node \
  target/node.live-workflow-bundle.preserves \
  --gate-receipt target/node.live-workflow-bundle-gate.preserves --require-gate-receipt \
  --expected-node node:local --expected-topic node-control --expected-peer peer:operator \
  --operation status --receipt-out target/sender-node.live-workflow-bundle-apply.preserves
molten node live-workflow-bundle-reconcile \
  target/sender-node.live-workflow-bundle-apply.preserves \
  --ingress-receipt target/node.status-ingress-deliver.preserves \
  --queue-receipt target/node.status-queue.preserves \
  --control-receipt target/node.status-dispatch.preserves \
  --receipt-out target/node.live-workflow-bundle-reconcile.preserves
molten node live-workflow-bundle-ack-export \
  target/sender-node.live-workflow-bundle-apply.preserves \
  --ingress-receipt target/node.status-ingress-deliver.preserves \
  --queue-receipt target/node.status-queue.preserves \
  --control-receipt target/node.status-dispatch.preserves \
  --reconcile-receipt target/node.live-workflow-bundle-reconcile.preserves \
  --out target/node.live-workflow-bundle-ack.preserves \
  --receipt-out target/node.live-workflow-bundle-ack-export.preserves
molten node live-workflow-bundle-ack-import --state-root target/sender-node \
  target/node.live-workflow-bundle-ack.preserves \
  --receipt-out target/sender-node.live-workflow-bundle-ack-import.preserves
molten node live-workflow-bundle-protocol-gate target/node.live-workflow-bundle.preserves \
  --gate-receipt target/node.live-workflow-bundle-gate.preserves \
  --apply-receipt target/sender-node.live-workflow-bundle-apply.preserves \
  --reconcile-receipt target/node.live-workflow-bundle-reconcile.preserves \
  --ack target/node.live-workflow-bundle-ack.preserves \
  --receipt-out target/node.live-workflow-protocol-gate.preserves
molten node live-workflow-bundle-import --state-root target/sender-node \
  target/node.live-workflow-bundle.preserves \
  --expected-node node:local --expected-topic node-control --expected-peer peer:operator \
  --operation status --receipt-out target/sender-node.live-workflow-bundle-import.preserves
molten node live-workflow-bundle --state-root target/node \
  --ticket target/node.bound-live-ticket.preserves \
  --peer-admission target/node.operator-peer-admission.preserves \
  --authority-grant target/node.operator-grant.preserves \
  --send-receipt target/node.status-live-send.preserves \
  --receive-receipt target/node.status-live-receive.preserves \
  --listener-receipt target/node.live-listener.preserves \
  --service-receipt target/node.service-run.preserves \
  --receipt-out target/node.live-workflow.preserves
molten node control-dispatch --state-root target/node --receipt-out target/node.status-dispatch.preserves
molten node status --state-root target/node --health-out target/node.health.preserves
molten node control-history --state-root target/node
molten node stop --state-root target/node --shutdown-out target/node.shutdown.preserves
molten node show target/node/startup-receipt.preserves
```

`init` writes canonical `node-config-v1` and node identity receipts under the explicit state root. `run` validates the source gate, starts required adapters in deterministic dependency order, writes an active startup-bound node lock, and emits `node-startup-receipt-v1` plus adapter receipts. `control-submit` persists canonical requests in the explicit state-root inbox and writes queue receipts; `control-ingress-build`/`publish`/`deliver` provide deterministic local-Iroh ingress that validates peer bootstrap, authority, policy, resource, and delivery-idempotency evidence before enqueue; `control-ingress-live-build` and `control-ingress-live-loopback` exercise the real `iroh-gossip` transport with canonical envelope bytes and live transport receipts while still feeding the durable ingress path, where live peer bootstrap refs must resolve to admitted `node-control-live-peer-admission-v1` ticket evidence and live authority refs must resolve to admitted `node-control-authority-grant-v1` ledger artifacts. `live-ticket-export` and `live-peer-admit` provide deterministic peer bootstrap tickets/admissions; `live-ticket-import` imports receiver tickets and optional peer-admission receipts into another explicit state root after node/topic/endpoint/peer/freshness validation; `authority-grant-fixture` emits/imports deterministic grants for local live-ingress workflows, and `authority-grant-import` imports moved grants after peer/node/operation/scope/epoch/revocation validation. Import receipts are operational evidence only: they do not replace peer admissions, grants, policy/resource, or provenance refs. Transport identity and neighbor evidence never count as bootstrap or authority. `control-ingress-live-send` uses a bound live ticket from `serve --live-ticket-out` to join the receiver's real `iroh-gossip` topic, publish canonical live ingress bytes, and emit `node-control-live-send-receipt-v1` plus live transport receipts without satisfying authority or provenance by transport alone. It also exposes an `--operation-id` guard over the derived envelope operation ref, `--expected-node`/`--expected-topic`/`--expected-endpoint` ticket guards, sender-state-root preflight for imported peer admissions and authority grants, bounded `--max-attempts`, canonical live-send retry receipts, and duplicate-send receipts that suppress repeat broadcasts when a prior pass send receipt already exists. Missing imports, wrong ticket bindings, unsupported addresses, operation mismatches, and join/publish failures are classified in live-send receipt checks (`receiver-ticket-expected`, `receiver-address-supported`, `operation-id-bound`, `sender-state-root-evidence`, `join-or-publish-succeeded`) with deterministic import guidance. `live-workflow-bundle-export` packages a ticket, peer admission, authority grant, and supporting receipts into a canonical `node-control-live-workflow-bundle-v1` handoff artifact; `live-workflow-bundle-verify` checks the handoff offline and emits a non-authority verify receipt; `live-workflow-bundle-gate` rechecks the bundle and optionally requires a current matching verify receipt before import; `live-workflow-bundle-apply` revalidates the gate, imports ticket/admission/grant members, optionally dry-runs a request preflight, and only sends over live Iroh when `--send` is explicitly provided while emitting a non-authority apply receipt; `live-workflow-bundle-reconcile` checks apply/send evidence against receiver ingress, queue, and optional control receipts so operators can prove enqueue/dispatch outcomes without making reconcile receipts authoritative; `live-workflow-bundle-ack-export` packages reconcile plus receiver evidence into a portable non-authority ack bundle, and `live-workflow-bundle-ack-import` imports that ack into the sender ledger after ref/operation/request guards; `live-workflow-bundle-protocol-gate` projects bundle handoff, apply evidence, and ack evidence through a finite Trellis sender/receiver protocol and emits a `protocol-session-gate-receipt-v1` that also denies on failed or mismatched workflow evidence while remaining non-authority/non-provenance; `live-workflow-bundle-import` validates that handoff and imports the underlying ticket/admission/grant artifacts into the sender state root while keeping bundle receipts non-authority. `live-workflow-bundle` ties ticket, peer admission, authority grant, send, receive/listener, and service-run receipts into a canonical `node-control-live-workflow-receipt-v1` operator runbook receipt. `supervisor-policy-fixture` emits/imports canonical `node-control-supervisor-policy-v1` bounds for restart attempts, restart windows, heartbeat timeouts, shutdown drain, and stale-lock recovery. `serve` acquires a separate service lock, emits service heartbeat/run receipts, scans local-Iroh ingress in deterministic order, and drains via the existing bounded control loop until the tick bound or shutdown stop; optional `--supervisor-policy` imports the policy, emits supervisor receipts for restart admission/denial, stale-lock recovery, duplicate-runner denial, and shutdown drain bounds, and fails closed when a stale service lock lacks recovery policy or restart bounds are exceeded; `serve --live-iroh` writes `--live-ticket-out` as soon as the endpoint is bound, so a separate peer can send while the listener waits, then runs a bounded live Iroh listener, records neighbor/session observations and live transport receipts, then drains through the same supervised control loop; `control-dispatch` requires the active lock, emits dispatch receipts, routes `status`/`shutdown`, and dispatches side-effecting `install`/`run`/`gate` operations through ledger-resolved payloads before importing operation subreceipts into the node ledger. `control-request` accepts explicit `--evidence` refs, and `provenance-fixture` emits a synthetic reviewed `provenance-record-v1` for local tests. `run-loop` drains the inbox in deterministic path order up to `--max-requests`, emits heartbeat and loop receipts, returns prior receipts for duplicate request refs, and stops after a passing shutdown dispatch removes the active lock. `install` writes node-control artifacts into the node registry only after reviewed/reproducible/policy-trusted provenance for the payload ref passes; `run` executes a node-local job execution request only after admitted provenance for the job ref passes; and `gate` validates strict Octet source-gate evidence for the target subject. Missing peer bootstrap, live ticket admission, authority, policy, resource, provenance, operation-required target/payload, duplicate service lock, live delegation grant, or ledger evidence fails closed before enqueue or operation side effects. `control-history` is a read-only listing of the node's latest delivery outcome per received envelope, its pending inbox requests, and its processed requests with their control-receipt decisions. `status` and `stop` are convenience paths over local Preserves control requests whose rendered text is non-normative; the canonical evidence is the emitted service/ingress/queue/control/health/shutdown/provenance/authority receipts imported into the node ledger.

## Iroh endpoint identity persistence

Node startup selects Iroh endpoint key material through a pure resolution core before shell-owned secret effects: explicit key, managed secret backend, restricted persisted state file, policy-admitted generate-and-persist, or fail-closed denial. Receipts bind public endpoint identity, redacted source metadata, permission diagnostics, and admitted rotation refs; they do not expose private key bytes, bearer tokens, raw tickets, or sensitive paths. Endpoint identity is transport continuity evidence only and never replaces peer admission, capability, authority, policy, resource, provenance, source-gate, retention, or execution checks.

## Delivery idempotency diagnostics

The delivery idempotency UX exposes canonical operation identity and dedup checks without running a full live transport loop:

```sh
molten test delivery scope --scope-profile remote-dataspace-topic \
  --scope-name peer:b:services --retention-ref blake3:policy \
  --out target/delivery.scope.preserves
molten test delivery operation-id --scope-profile remote-dataspace-topic \
  --scope-name peer:b:services --producer peer:a/producer --consumer peer:b \
  --sequence 1 --intent remote-dataspace-assert --payload-ref blake3:payload \
  --policy-ref blake3:policy --out target/delivery.operation.preserves
molten test delivery check --root target/delivery-store \
  --scope-profile remote-dataspace-topic --scope-name peer:b:services \
  --producer peer:a/producer --consumer peer:b --sequence 1 \
  --intent remote-dataspace-assert --payload-ref blake3:payload \
  --policy-ref blake3:policy --evidence-ref blake3:evidence \
  --semantic-result-ref blake3:result \
  --receipt-out target/delivery.first.preserves
```

`first` receipts permit the caller to commit its side effect; exact duplicates emit `duplicate` receipts with side effect `suppress` and a prior receipt ref; conflicts, stale sequences, and gaps deny or retry before side effects. These receipts are replay/dedup evidence only and do not grant transport, authority, provenance, policy, resource, or execution trust.

## Upgrade session protocol drains

Upgrade `drain-sessions` tasks now bind protocol lifecycle evidence before task completion. A passing drain requires a ledger-resolved `protocol-session-gate-receipt-v1` for the affected old protocol artifact, a `pass` decision, non-empty terminal session-state refs, and a protocol ref matching the task's `from_ref` (or fallback affected/compatibility refs). Missing, denied, stale, or wrong-protocol gate receipts emit deny `upgrade-receipt-v1` diagnostics before cutover side effects. These drain receipts are upgrade coordination evidence only; they do not grant protocol authority, transport, policy, resource, provenance, or execution trust.

## Retention and GC pinning diagnostics

Retention diagnostics make deletion eligibility explicit before any object is removed, redacted, tombstoned, or compacted:

```sh
molten test retention class --class-name private-secret-ref \
  --deletion-authority-ref blake3:authority --policy-ref blake3:policy \
  --secret-redaction-hook true --remote-gc-plan true \
  --out target/retention.class.preserves
molten test retention pin --root target/retention-store \
  --object-ref blake3:object --object-kind encrypted-ref \
  --retention-class private-secret-ref --source secret-redaction \
  --reason repro-reveal-pending --owner-ref blake3:owner \
  --policy-ref blake3:policy --evidence-ref blake3:evidence \
  --pin-out target/retention.pin.preserves \
  --receipt-out target/retention.pin-receipt.preserves
molten test retention admit --root target/retention-store \
  --kind authority --requester-ref blake3:owner \
  --object-ref blake3:object --object-kind encrypted-ref \
  --retention-class private-secret-ref --action delete \
  --bound-ref blake3:authority-grant --reference-index-complete \
  --out target/retention.authority-admission.preserves
molten test retention remote-clearance --root target/retention-store \
  --requester-ref blake3:owner --peer-ref blake3:peer \
  --object-ref blake3:object --object-kind encrypted-ref \
  --retention-class private-secret-ref --action delete \
  --remote-ref blake3:remote-cache --policy-ref blake3:policy \
  --authority-ref blake3:authority-grant --evidence-ref blake3:evidence \
  --out target/retention.remote-clearance.preserves
molten test retention remote-clearance-request --root target/retention-store \
  --requester-ref blake3:owner --peer-ref blake3:peer \
  --object-ref blake3:object --object-kind encrypted-ref \
  --retention-class private-secret-ref --action delete \
  --remote-ref blake3:remote-cache --policy-ref blake3:policy \
  --authority-ref blake3:authority-grant --evidence-ref blake3:evidence \
  --out target/retention.remote-clearance-request.preserves
molten test retention remote-clearance-respond --root target/peer-retention-store \
  target/retention.remote-clearance-request.preserves \
  --evidence-ref blake3:peer-reference-index \
  --out target/retention.remote-clearance-response.preserves
molten test retention remote-clearance-import --root target/retention-store \
  --request target/retention.remote-clearance-request.preserves \
  --response target/retention.remote-clearance-response.preserves \
  --expected-peer-ref blake3:peer --expected-remote-ref blake3:remote-cache \
  --out target/retention.remote-clearance-import.preserves
molten test retention remote-clearance-live-loopback --root target/retention-store \
  --requester-node-root target/requester-node --peer-node-root target/peer-node \
  --requester-node-id requester-node --peer-node-id peer-node \
  --requester-ref blake3:owner --peer-ref blake3:peer \
  --object-ref blake3:object --object-kind encrypted-ref \
  --retention-class private-secret-ref --action delete \
  --remote-ref blake3:remote-cache --policy-ref blake3:policy \
  --authority-ref blake3:authority-grant \
  --retention-evidence-ref blake3:evidence \
  --request-peer-bootstrap-ref blake3:requester-peer-admission \
  --request-authority-ref blake3:request-authority-grant \
  --request-policy-ref blake3:request-policy \
  --request-resource-ref blake3:request-resource \
  --response-peer-bootstrap-ref blake3:peer-admission \
  --response-authority-ref blake3:response-authority-grant \
  --response-policy-ref blake3:response-policy \
  --response-resource-ref blake3:response-resource \
  --request-out target/retention.live-request.preserves \
  --response-out target/retention.live-response.preserves \
  --import-out target/retention.live-import.preserves \
  --receipt-out target/retention.live-workflow.preserves
molten test retention remote-clearance-live-request-send --root target/retention-store \
  --peer-ticket target/peer.live-ticket.preserves \
  --requester-node-id requester-node --peer-node-id peer-node \
  --requester-ref blake3:owner --peer-ref blake3:peer \
  --object-ref blake3:object --object-kind encrypted-ref \
  --retention-class private-secret-ref --action delete \
  --remote-ref blake3:remote-cache --policy-ref blake3:policy \
  --authority-ref blake3:authority-grant \
  --request-out target/retention.live-request.preserves \
  --control-out target/retention.live-request-control.preserves \
  --receipt-out target/retention.live-request-send.preserves
molten test retention remote-clearance-live-response-send --root target/peer-retention-store \
  --requester-ticket target/requester.live-ticket.preserves \
  target/retention.live-request.preserves \
  --peer-node-id peer-node --requester-node-id requester-node \
  --response-evidence-ref blake3:peer-reference-index \
  --response-out target/retention.live-response.preserves \
  --control-out target/retention.live-response-control.preserves \
  --receipt-out target/retention.live-response-send.preserves
molten test retention remote-clearance-live-import-workflow --root target/retention-store \
  --request target/retention.live-request.preserves \
  --response target/retention.live-response.preserves \
  --request-control target/retention.live-request-control.preserves \
  --request-send-receipt target/retention.live-request-send.preserves \
  --request-receive-receipt target/peer.request-receive.preserves \
  --request-ingress-ref blake3:peer-ingress \
  --response-control target/retention.live-response-control.preserves \
  --response-send-receipt target/retention.live-response-send.preserves \
  --response-receive-receipt target/requester.response-receive.preserves \
  --response-ingress-ref blake3:requester-ingress \
  --expected-peer-ref blake3:peer --expected-remote-ref blake3:remote-cache \
  --import-out target/retention.live-import.preserves \
  --receipt-out target/retention.live-workflow.preserves
molten test retention explain --root target/retention-store \
  --object-ref blake3:object --object-kind encrypted-ref \
  --retention-class private-secret-ref --action delete --subsystem ledger-gc \
  --out target/retention.explain.preserves
molten test retention bundle-export --root target/retention-store \
  --explain target/retention.explain.preserves --out target/retention-bundle \
  --profile public
molten test retention bundle-verify --bundle target/retention-bundle \
  --receipt-out target/retention-bundle.verify.preserves
molten test retention gc-plan --root target/retention-store \
  --subsystem ledger-gc --object-ref blake3:object --object-kind encrypted-ref \
  --retention-class private-secret-ref --action delete \
  --retention-requester blake3:owner \
  --retention-policy-ref blake3:policy \
  --retention-authority-ref blake3:authority-grant \
  --retention-evidence-ref blake3:evidence \
  --retention-reference-index-ref blake3:reference-index \
  --retention-remote-gc-ref blake3:remote-gc \
  --retention-remote-clearance-ref blake3:remote-clearance \
  --retention-reference-index-complete \
  --out target/retention.gc-plan.preserves
molten test retention gc-apply-plan --root target/retention-store \
  --plan-ref blake3:retention-gc-plan \
  --receipt-out target/retention.gc-apply.preserves
molten test retention gc-audit --root target/retention-store \
  --execution-ref blake3:retention-gc-execute \
  --out target/retention.gc-audit.preserves
molten test retention check --root target/retention-store \
  --object-ref blake3:object --object-kind encrypted-ref \
  --retention-class private-secret-ref --action delete \
  --requester-ref blake3:owner --reference-index-complete true \
  --policy-ref blake3:policy --evidence-ref blake3:evidence \
  --has-delete-authority true --remote-gc-clearance \
  --receipt-out target/retention.delete.preserves
```

Pinned objects, legal holds, retained receipt dependencies, remote/cache uncertainty, incomplete reference indexes, and missing requester/policy/authority/supporting evidence all deny before destructive side effects. `retention explain` emits a read-only `retention-candidate-explain-v1` artifact for an object ref, listing known pins, evidence admissions, remote clearances/imports, GC plans, applies, executions, audits, retention receipts, and tombstones before an operator chooses a destructive command; it is discovery evidence only. `retention bundle-export` writes a handoff directory containing the explain artifact, a canonical `retention-candidate-bundle-v1`, and the referenced local plan/apply/execute/audit/receipt/tombstone artifacts for review without granting authority; `--profile internal|public|diagnostic` records `retention-candidate-bundle-profile-v1`, with public deny-sensitive decisions and diagnostic redacted review copies under `redacted/`. `retention bundle-verify` emits `retention-candidate-bundle-verify-v1` evidence after recomputing packaged artifact refs and detecting missing, tampered, duplicate, or unreferenced bundle files; verification remains review evidence only. `retention gc-plan` emits a dry-run `retention-gc-plan-v1` artifact that lists requester, policy, authority, supporting-evidence, reference-index, local-retention, remote-GC, remote-clearance, and evidence-only boundary gates without writing retention receipts, tombstones, or deleting content. `retention gc-apply-plan` requires a stored plan ref, recomputes the plan from its embedded candidate/evidence before mutation, denies on drift or failed admission without writing destructive retention receipts or tombstones, and emits `retention-gc-apply-v1` evidence linking the original plan, recomputed plan, admitted evidence refs, retention receipt ref, and tombstone ref. Evidence-ledger GC, chunk-store GC, evaluation-cache invalidation, and secret cleanup bind retention receipts before removing content or writing tombstones; denial leaves content intact and emits auditable subsystem receipts. Non-dry-run ledger GC, chunk GC, and cache invalidation additionally require per-candidate `--apply-ref` values from passing `retention-gc-apply-v1` receipts; each candidate emits `retention-gc-execute-v1` evidence that binds subsystem/action/object/class, apply plan, retention receipt, and tombstone refs before mutation while still rerunning normal destructive admission. `retention gc-audit` follows a stored execution gate and emits `retention-gc-audit-v1` operator evidence showing the plan → apply → execute → retention receipt → tombstone chain, with consistency diagnostics and no new trust or deletion authority. Catalog and MCP read-only search classify retention GC plan/apply/execute/audit artifacts by `retention-gc:*` markers, object ref, subsystem, decision, and chain refs; MCP clients can use `search_retention_gc` for operator discovery without gaining deletion authority. Ledger GC, chunk GC, and cache invalidation accept explicit `--retention-requester`, `--retention-policy-ref`, `--retention-authority-ref`, `--retention-evidence-ref`, `--retention-retained-ref`, `--retention-remote-peer-ref`, `--retention-remote-ref`, `--retention-reference-index-ref`, `--retention-remote-gc-ref`, `--retention-remote-clearance-ref`, and `--retention-reference-index-complete` inputs; apply-mode candidates without matching local `retention-evidence-admission-v1` receipts, per-remote `retention-remote-gc-clearance-v1` receipts, and matching apply refs fail closed. The remote-clearance request/respond/import commands let a requester send a canonical request to a peer, let that peer embed its locally evaluated `retention-remote-gc-clearance-v1` value in a response, and import only passing scope-matching peer clearance into the local retention store. The live loopback command carries the same request and response refs over node-control live transport receipts in both directions, while the multi-host live request-send, response-send, and import-workflow commands split that flow across two operator-managed node roots and explicit send/receive/ingress receipts. Both live paths still require the local import gate before destructive admission can use the embedded peer clearance. Admission receipts bind requester, object, class, action, evidence kind, current/revoked state, reference-index proof, local remote-GC plan, and per-peer remote clearance before mutation. Passing destructive actions emit retention receipts and tombstone/redaction metadata that preserve audit context without leaking private content. Retention, remote-clearance, and live workflow receipts are deletion-safety evidence only and do not grant authority, provenance, transport, policy, resource, source-gate, remote-GC, or execution trust.

Retention GC audit chains imported into the local ledger can be discovered through the catalog without mutation:

```sh
molten test catalog search --registry target/catalog --ledger target/ledger \
  --text retention-gc-object:blake3:object
molten test catalog search --registry target/catalog --ledger target/ledger \
  --ledger-kind retention-gc-audit --text retention-gc:audit
molten test catalog mcp-call target/search-retention-gc.preserves \
  --registry target/catalog --ledger target/ledger \
  --out target/search-retention-gc.response.preserves \
  --receipt-out target/search-retention-gc.receipt.preserves
```

`search-retention-gc.preserves` is a read-only MCP request for `search_retention_gc` with optional `stage`, `object-ref`, `subsystem`, `decision`, `plan-ref`, `apply-ref`, and `execution-ref` arguments.

`molten dogfood local-node` now includes a local retention GC review chain under the dogfood state root: fixture evidence admissions and remote clearance, dry-run plan, apply, execute, audit, candidate explain, public bundle export/profile/verify, and `search_retention_gc` MCP discovery. These dogfood refs are release review evidence only; destructive subsystems still require normal retention admission, plan/apply/execute, remote clearance, receipt, and tombstone gates.

## Supply-chain provenance diagnostics

The provenance UX exposes canonical trust-state records and evaluation receipts without running a full node-control dispatch:

```sh
molten test provenance fixture --artifact-ref blake3:artifact \
  --out target/provenance.reviewed.preserves
molten test provenance record --artifact-ref blake3:artifact --trust-state reviewed \
  --source-ref blake3:source --dependency-closure-ref blake3:deps \
  --toolchain-ref blake3:toolchain --builder-ref blake3:builder \
  --review-ref blake3:review --test-ref blake3:tests \
  --source-gate-ref blake3:octet --policy-ref blake3:policy \
  --out target/provenance.record.preserves
molten test provenance evaluate --operation install --profile node-control \
  --artifact-ref blake3:artifact --provenance target/provenance.record.preserves \
  --receipt-out target/provenance.receipt.preserves
molten test provenance show target/provenance.receipt.preserves
molten test provenance build-record --expected-artifact-ref blake3:artifact \
  --source-ref blake3:source --dependency-closure-ref blake3:deps \
  --toolchain-ref blake3:toolchain --build-param target=x86_64-linux \
  --builder-ref blake3:builder --nix-derivation-ref blake3:drv \
  --policy-ref blake3:policy --evidence-ref blake3:octet \
  --out target/provenance.build-record.preserves
molten test provenance verify-build target/provenance.build-record.preserves \
  --actual-artifact-ref blake3:artifact \
  --receipt-out target/provenance.build-verify.preserves
molten test provenance record --artifact-ref blake3:artifact \
  --trust-state reproducible-verified --source-ref blake3:source \
  --dependency-closure-ref blake3:deps --toolchain-ref blake3:toolchain \
  --builder-ref blake3:builder --build-record-ref blake3:build-record \
  --out target/provenance.reproducible.preserves
molten test provenance evaluate --operation install --profile node-control \
  --artifact-ref blake3:artifact --provenance target/provenance.reproducible.preserves \
  --build-verification target/provenance.build-verify.preserves \
  --receipt-out target/provenance.reproducible.receipt.preserves
```

Provenance receipts explain whether an artifact's explicit provenance is admitted for a profile; build verification receipts explain whether a reproducible build record's expected artifact matches the actual artifact. A `reproducible-verified` provenance record is denied unless a matching passing build verification receipt references a build record ref bound by that provenance record. Sensitive operations such as policy-artifact, migration-recipe, production-executable, and remote execution admission require stronger `reproducible-verified` or `policy-trusted` provenance. Job `sync-loopback` validates supplied `--provenance` / `--build-verification` evidence before installing missing remote-sync artifacts, and catalog/MCP read-only views classify provenance records and receipts (`list_provenance`, `search_provenance`). They remain evidence only: authority, policy, resource, transport, execution, and source-gate checks must still be supplied independently.

## Blob-ref job submission

Ref-backed jobs are exposed under `molten test job` so executable, input, output, and receipt identity is content-addressed before worker execution:

```sh
molten test job ref-submit --job-id job:echo --operation-id blake3:operation \
  --executable blake3:executable-manifest@12@elf-executable \
  --input blake3:input-manifest@1024@bytes \
  --context-ref blake3:authority --policy-ref blake3:policy \
  --provenance-ref blake3:provenance --effect-ref blake3:effects \
  --out target/job-ref.submission.preserves
molten test job ref-execute target/job-ref.submission.preserves \
  --chunks target/chunks --ledger target/ledger \
  --receipt-out target/job-ref.receipt.preserves
molten test job status --ledger target/ledger --job job:echo
```

`job-ref-submission-v1` is content-ref-only and rejects inline executable/input bytes. The local deterministic worker reads and verifies chunk manifests before running the `local-echo-v1` handler, pins executable/input/output refs while active, emits fetch/verify/status/cleanup evidence, stores outputs as chunk manifests, and records `job-ref-receipt-v1` in the ledger. These receipts are execution evidence only; authority, provenance, policy, effect admission, transport, and resource trust remain explicit inputs.

## Job DAG Iroh worker UX

Recorded local-gossip worker execution requires sync, target admission, an execution request, and a generation-bound local receiver recording. Use already initialized sender/receiver node roots whose actual node IDs match `SOURCE_PEER`/`TARGET_PEER`; see [preparing a recorded exchange](../job-workflows/preparing-a-recorded-worker-exchange.md). All reference variables below come from reviewed canonical artifacts, not invented digests. Choose either worker execution variant: running both can execute the workload twice.

```sh
molten test job worker-request \
  --admission-receipt target/job-admission.receipt.preserves \
  --execution-request target/job-execution.request.preserves \
  --target-peer "${TARGET_PEER:?Actual receiver node ID}" \
  --authority-ref "${AUTHORITY_REF:?Actual authority evidence}" \
  --resource-ref "${RESOURCE_REF:?Actual resource evidence}" \
  --peer-bootstrap-ref "${PEER_BOOTSTRAP_REF:?Actual bootstrap evidence}" \
  --node-identity-ref "${NODE_IDENTITY_REF:?Actual identity evidence}" \
  --out target/job-worker.request.preserves
molten test remote session admit \
  --sender-state-root "${SENDER_STATE:?Initialized sender root}" \
  --receiver-state-root "${RECEIVER_STATE:?Initialized receiver root}" \
  --topic molten.job.worker --receipt-out target/job-session-admit.preserves
# Review that canonical admission receipt and set SESSION_GENERATION from it.
# JOB_REF and EXECUTION_REQUEST_REF are the operands in the reviewed worker request.
printf '<job-worker-operands-v1 "%s" "%s">\n' \
  "${JOB_REF:?Actual admitted job ref}" "${EXECUTION_REQUEST_REF:?Actual execution request ref}" \
  > target/job-worker-introduction.preserves
molten test remote session commit \
  --sender-state-root "$SENDER_STATE" --receiver-state-root "$RECEIVER_STATE" \
  --topic molten.job.worker --generation "${SESSION_GENERATION:?Actual admitted generation}" \
  --operation assert --payload target/job-worker-introduction.preserves \
  --receipt-out target/job-worker-introduction.receipt.preserves
molten test job worker-run-local target/job-worker.request.preserves \
  --target-registry target/job-target-registry \
  --storage target/job-worker-storage --cache target/job-worker-cache \
  --admission-receipt target/job-admission.receipt.preserves \
  --execution-request target/job-execution.request.preserves \
  --from-peer "${SOURCE_PEER:?Actual sender node ID}" \
  --sender-state-root "$SENDER_STATE" --receiver-state-root "$RECEIVER_STATE" \
  --session-generation "$SESSION_GENERATION" \
  --transport-root target/job-worker-transport --ledger target/ledger \
  --out target/job-worker-run
molten test job worker-schedule-local target/job-worker.request.preserves \
  --target-registry target/job-target-registry \
  --storage target/job-scheduled-storage --cache target/job-scheduled-cache \
  --admission-receipt target/job-admission.receipt.preserves \
  --execution-request target/job-execution.request.preserves \
  --from-peer "$SOURCE_PEER" \
  --sender-state-root "$SENDER_STATE" --receiver-state-root "$RECEIVER_STATE" \
  --session-generation "$SESSION_GENERATION" \
  --transport-root target/job-scheduled-transport \
  --coordination-authority-ref "$AUTHORITY_REF" \
  --coordination-resource-ref "$RESOURCE_REF" \
  --coordination-policy-ref "${COORDINATION_POLICY_REF:?Actual scheduling policy evidence}" \
  --ledger target/ledger --out target/job-worker-scheduled
```

`worker-run-local` writes the worker request, remote dataspace envelope, publish/delivery receipts, replayable delivery log, assignment, status records, result, worker receipt, execution receipt, and output evidence. `worker-schedule-local` wraps that path with a coordination queue enqueue/dequeue, duplicate enqueue operation replay, lock/fencing-token lease acquisition, stale-token denial before worker execution, release, and a `job-worker-schedule-receipt-v1`. Worker transport, queue claims, lease tokens, and CLI receipts are evidence only; peer identity or message delivery does not grant authority, policy, resource, provenance, source-gate, or execution trust.

Source admission establishes only local identity/session bootstrap. The receiver must actually commit the introducing assertion; carrying the same refs in authority, evidence, or identity metadata does not introduce workload operands. Retraction, close, changed identities, or a stale generation refuse the next referenced build/publication. A local recording is not a live-network receiver acknowledgment and never grants job execution authority.

## Coordination control-plane UX

Coordination manifests and requests can be generated and applied without bypassing the control-plane state machine:

```sh
molten test coordination manifest --service queue \
  --policy-ref blake3:policy --resource-ref blake3:resource \
  --out target/coordination.manifest.preserves
printf '<item "job-1">' > target/coordination.item.preserves
molten test coordination request --service queue --operation enqueue \
  --key queue:jobs --client-session worker-a \
  --operation-id-ref blake3:operation --payload target/coordination.item.preserves \
  --authority-ref blake3:authority --policy-ref blake3:policy \
  --resource-ref blake3:resource --out target/coordination.request.preserves
molten test coordination apply --manifest target/coordination.manifest.preserves \
  --request target/coordination.request.preserves --out target/coordination-apply
molten test coordination show target/coordination-apply/report.preserves
```

`apply` writes `coordination-apply-report-v1` plus indexed request/receipt/state/assertion evidence. Duplicate operation ids replay the prior receipt instead of advancing state twice. Coordination receipts and reports are evidence only; they do not grant authority, policy, resource, transport, provenance, or source-gate trust.

## Production readiness receipts

The production readiness runbooks live in [`docs/production-operator-runbooks.md`](../../production-operator-runbooks.md), with a typed Nickel deployment profile at [`docs/production-node-profile.ncl`](../../production-node-profile.ncl). Profile export requires an explicit reviewed `candidate_source_ref`; the checked-in source contains no release candidate placeholder.

`molten test prod-soak` now also emits canonical production readiness receipts for deployment profiles, backup/restore drills, upgrade/rollback drills, observability/SLO snapshots, runbook checks, security threat models, security drills, redaction audits, supply-chain reviews, boundary negative suites, incident response drills, security readiness reports, pilot decisions, and release-candidate gates. A release-candidate gate requires every evidence artifact to declare the same reviewed candidate source. These receipts validate declared identity bindings only; they do not prove external artifact truth or grant authority, policy, provenance, retention, transport, source-gate, destructive-operation, or release trust. A release candidate with only configuration-clean Octet evidence must carry a source-gate caveat and can only support an explicitly scoped pilot decision.

## Testing evidence gates

Deterministic drift checks compare canonical evidence refs, not rendered logs. Use `molten test drift compare --workflow NAME --left A.preserves --right B.preserves` for artifact-level checks, or `molten test drift compare-summary` with `--left-ref path=blake3:...`, `--right-ref path=blake3:...`, and explicit `--variance path=temporary-root|runtime-path|store-path|diagnostic-log|rendered-output` declarations for non-semantic fields. Retry success is not accepted as proof that drift was absent.

Distributed simulation evidence is described in [`docs/distributed-testing.md`](../../distributed-testing.md). Simulation fault plans provide fast deterministic checks for delay, drop, duplicate, reorder, partition, rejoin, crash, restart, stale evidence, and ambient-state drift before VM or live soak evidence is needed.

Requirement traceability is available with `molten test traceability scan --root . --changed-only --coverage 'REQ|positive|tests/path.rs|cargo test name|blake3:...' --coverage 'REQ|negative|tests/path.rs|cargo test name|blake3:...'`. The checked-in matrix source is `tests/evidence-matrix.ncl`; the pure matrix gate emits `evidence-matrix-v1` diagnostics for missing positive coverage, missing negative coverage, stale ids, duplicates, missing refs, unsupported scopes, and explicit diagnostic-only exemptions. Effective configuration readback is available with `molten test traceability effective-config --profile-ref blake3:... --field 'node.id|node:local|profile|blake3:...|' --out target/effective-config.preserves --summary-out target/effective-config.txt`; field sources use `profile`, `cli-override`, `environment`, `ledger`, or `default`, and summaries are diagnostic views over the canonical `effective-config-readback-v1` artifact. Operator context profile expansion is available with `molten test traceability context-profile --profile-id operator:node-control --allowed-operation node.install --operation node.install --policy-ref blake3:... --authority-ref blake3:... --resource-ref blake3:... --require-policy --require-authority --require-resource --out target/context-profile.preserves`; the resulting `context-profile-expansion-v1` receipt binds the profile and expanded refs but does not grant authority. Boundary coverage, replay-smoke, generated tamper matrix, Hegel counterexample fixture, semantic nextest profile, CLI receipt-first, effective-config readback, operator context profile, config portability, and canonical CI test-run receipt cores live under pure Molten modules; their receipts are evidence-only and rendered output remains diagnostic-only.

NixOS VM evidence is validated by canonical receipts and preserved through the `nixos-vm-multinode` check output. Executable fault evidence is documented in [`docs/nixos-vm-executable-faults.md`](../../nixos-vm-executable-faults.md). Inspect the realized output's `vm-evidence/vm-evidence-manifest.preserves`, `vm-evidence/vm-evidence-validation.preserves`, `topology.preserves`, node evidence receipts, `vm-test-run.preserves`, and `prod-soak-run.preserves`; terminal, QEMU, and systemd logs are diagnostic-only and cannot override a canonical deny receipt.


## Deterministic harness and runtime examples

First deterministic harness/runtime slice:

```sh
cargo run -- test run examples/two-actor.preserves --report-out target/molten-reports/two-actor.report.preserves
cargo run -- test report validate target/molten-reports/two-actor.report.preserves
cargo run -- test replay target/molten-reports/two-actor.report.preserves
cargo run -- test report show target/molten-reports/two-actor.report.preserves
cargo run -- test gate check target/molten-reports/two-actor.report.preserves --receipt-out target/molten-reports/two-actor.gate-receipt.preserves
cargo run -- test report show target/molten-reports/two-actor.gate-receipt.preserves
cargo run -- test repro export target/molten-reports/two-actor.report.preserves --out target/molten-repro/two-actor
cargo run -- test repro verify target/molten-repro/two-actor/refs.preserves --receipt-out target/molten-repro/two-actor/verify-receipt.preserves
cargo run -- test repro unpack target/molten-repro/two-actor/refs.preserves --out target/molten-repro/two-actor-unpacked
cargo run -- test gate check target/molten-repro/two-actor/refs.preserves --receipt-out target/molten-repro/two-actor/gate-receipt.preserves
cargo run -- test run examples/executor-conformance.preserves --report-out target/molten-reports/executor-conformance.report.preserves
```

This runs a fresh local native two-actor suite through the Preserves harness rail and the in-process Molten runtime kernel, writes a canonical Preserves report with actor-registry, effect-log, policy-gate, capability-gate, budget-gate, admission-decision, executor hostcall boundary, and budget evidence, replays by injecting recorded clock/random effect responses to the same report/state refs, exports a sealed repro bundle with an embedded report gate receipt, and checks the report or sealed repro bundle as admissible pass evidence with `molten test gate check`. Harness steps now carry canonical Preserves values for message bodies, assertions, retractions, and exact-value observe patterns; string payloads are just the compatibility-friendly shorthand. Suites may include an optional static `<policy-v1 "molten.harness.policy.v1" [...]>` fixture with deny rules such as `<deny "producer" "assert" #f "service.ready" "producer cannot assert readiness">`, and must include explicit `<budget-v1 "molten.harness.budget.v1" ...>`, `<actor-registry-v1 "molten.harness.actor-registry.v1" [...]>`, and `<capabilities-v1 "molten.harness.capabilities.v1" [...]>` fixtures; actor registries bind ids to executor kinds without inferred actors or fallback execution, reviewed Steel hostcall actors may include explicit `<steel-executor-v1 ...>` source/callable/allowed-hostcall fixtures and execute in a reviewed Steel VM with canonical execution receipts, reviewed Wasm hostcall actors may include explicit `<wasm-executor-v1 ...>` module/WIT/allowed-hostcall fixtures validated with `wasmparser` and executed through a no-WASI Wasmtime hostcall shim with optional `molten.wasm.abi.v1` Preserves byte input/output, adapter/remote-proxy kinds require explicit executable preflight fixtures and verified transcript profiles before deterministic gates, capability grants such as `<grant "producer" "send" "consumer" #f>` deny by default when no matching grant exists, and omitted budget/actor/capability fixtures cannot execute or satisfy pass-evidence gates. Denied turns roll back and denied effects do not issue ambient effect requests. Before runtime turns or effects, the harness canonicalizes the policy, capability, and budget snapshots and emits `<policy-gate-v1 "molten.harness.policy-gate.v1" ...>`, `<capability-gate-v1 "molten.harness.capability-gate.v1" ...>`, and `<budget-gate-v1 "molten.harness.budget-gate.v1" ...>` evidence; policy gates now derive deterministic Nickel static source/export refs, validate a Basalt Nickel contract envelope/preflight receipt, and keep unreviewed Steel/dynamic predicate records fail-closed; capability gates validate a Basalt authority contract/preflight receipt, bind explicit empty local UCAN proofsets, and bind every grant ref used by admission authority evidence; budget gates derive deterministic Nickel resource-policy source/export refs and validate a Basalt resource preflight receipt. Report validation fails closed unless the embedded suite has explicit budget/actor/capability fixtures, report budget/actor-registry and policy/capability/budget gate evidence match the embedded suite, every declared actor has matching `<executor-preflight-v1 "molten.runtime.executor-preflight.v1" ...>` evidence with bound executor conformance suite refs, reviewed Steel actor fixtures carry `<steel-review-receipt-v1 ...>` source/callable/allowed-hostcall bindings and admitted Steel steps carry `<steel-execution-receipt-v1 ...>` VM/input/output evidence, reviewed Wasm actor fixtures carry `<wasm-inspection-receipt-v1 ...>` module/import/WIT/allowed-hostcall bindings and admitted Wasm steps carry `<wasm-execution-receipt-v1 ...>` fuel/memory/hostcall/ABI execution evidence, every observation has exactly one matching admission decision with authority evidence, the decision recomputes from embedded capabilities plus policy, actor activity is wrapped in matching `<actor-input-v1 ...>`, `<hostcall-request-v1 ...>`, `<hostcall-decision-v1 ...>`, and `<actor-output-v1 ...>` envelopes, denied turns contain only rollback evidence, and denied effects contain no effect records. Successful gate decisions emit canonical `<gate-receipt-v1 "molten.harness.gate-receipt.v1" ...>` Preserves artifacts containing artifact refs, validation/replay results, explicit-budget/no-default-resource-policy/actor-registry/no-inferred-actors/executor-boundary/executor-preflight/executor-conformance/Steel-review/Wasm-inspection/Wasm-execution/executor-hostcall-boundary/Nickel-policy-source/Nickel-export/Basalt-preflight/Basalt-authority/UCAN-proofset/Nickel-resource-policy/Basalt-resource/capability/hostcall/admission check evidence; without `--receipt-out`, the receipt is emitted on stdout. `molten test repro export --profile deny-sensitive` (the default) writes a sealed `<harness-repro-bundle-v1 ...>` refs artifact plus `gate-receipt.preserves`; the sealed bundle includes redaction policy/gate evidence and refuses sensitive Preserves marker records such as `<secret ...>`, `<confidential ...>`, `<credential ...>`, `<private ...>`, and unvalidated `<encrypted-ref ...>`. `--profile redacted-diagnostic` emits deterministic redaction markers plus canonical redaction transform manifest/receipt files and remains diagnostic-only: `repro verify` and `test gate check` fail closed unless a future policy explicitly marks the transform gate-preserving. `--profile encrypted-private` emits validated `<encrypted-ref-v1 ...>` placeholders and a private-bundle profile; unpacking requires passing reveal receipts via `--reveal-receipt` that bind the exact encrypted-ref ids in the bundle, while missing, stale, malformed, or unauthorized reveal evidence fails closed. `molten test repro verify` emits canonical `<repro-verify-receipt-v1 ...>` receipts for sealed pass bundles, `molten test repro unpack` materializes verified report/suite/receipt contents or diagnostic redacted evidence, and bundle gate checks recompute redaction evidence and embedded report receipts before emitting a new `repro-bundle` gate receipt, while failure repro bundles remain diagnostic-only.

SAM service supervision CLI slice:

```sh
cargo run -- service run-supervision-fixture --out target/molten-service-supervision
cargo run -- service show-supervision target/molten-service-supervision/report.preserves
cargo run -- service replay-supervision target/molten-service-supervision/report.preserves
cargo run -- service gate-supervision target/molten-service-supervision/report.preserves \
  --receipt-out target/molten-service-supervision/gate-receipt.preserves
```

These commands exercise demand-driven service failure supervision over canonical SAM service records. The supervision report carries failure markers, failed/final statuses, lifecycle receipts, deterministic monitor notifications, restart decisions, scheduled restart demand, cleanup receipts, owned-state retractions, and retention inputs as Preserves evidence. `gate-supervision` emits canonical `<service-supervision-gate-receipt-v1 "molten.service.supervision-gate-receipt.v1" ...>` receipts that replay the report, bind status/lifecycle/restart/monitor/cleanup evidence, and stay operational evidence only: they do not grant service authority, resource rights, provenance, or transport trust.

Trellis protocol session CLI slice:

```sh
cargo run -- test protocol run-request-response --out target/molten-protocol
cargo run -- test protocol gate-lifecycle target/molten-protocol \
  --receipt-out target/molten-protocol/gate-receipt.preserves
cargo run -- test protocol show target/molten-protocol/gate-receipt.preserves
```

`run-request-response` installs a finite Trellis-backed protocol manifest, writes endpoint/session/message/operation evidence, and advances projected client/server states through a canonical request/response exchange. `gate-lifecycle` emits `<protocol-session-gate-receipt-v1 "molten.protocol.session-gate-receipt.v1" ...>` after replaying the install receipt and operation receipts against the state/message evidence and checking terminal session state. The gate receipt is replay evidence only; it does not grant protocol authority, resource rights, provenance, policy admission, or transport trust.

Remote SAM/Iroh dataspace CLI slice:

```sh
cargo run -- test remote envelope build \
  --from-peer peer:a --from-actor producer --to-peer peer:b \
  --topic services --operation assert \
  --payload examples/remote-service-ready.preserves \
  --out target/molten-remote/envelope.preserves
cargo run -- test remote publish-local \
  --transport-root target/molten-remote/transport \
  --envelope target/molten-remote/envelope.preserves \
  --node peer:a \
  --receipt-out target/molten-remote/publish.preserves
cargo run -- test remote deliver-local \
  --transport-root target/molten-remote/transport \
  --topic services --envelope-ref blake3:... --receiver-peer peer:b \
  --out target/molten-remote/delivered.preserves \
  --receipt-out target/molten-remote/deliver.preserves
cargo run -- test remote run-two-peer \
  --transport-root target/molten-remote/transport \
  --out target/molten-remote/two-peer
cargo run -- test remote gate \
  --delivery-log target/molten-remote/two-peer/delivery-log.preserves \
  --admission-receipt target/molten-remote/two-peer/admission-receipt.preserves \
  --turn-context-ref $(cat target/molten-remote/two-peer/turn-context-ref.preserves | tr -d '\"') \
  --receipt-out target/molten-remote/two-peer/gate-receipt-2.preserves
```

These commands exercise canonical remote dataspace envelopes over deterministic `iroh-local-gossip`, record delivery logs for replay, and emit remote dataspace gate receipts. Live `iroh-gossip` uses the same library boundary, but unrecorded live timing is not deterministic pass evidence.

Failure artifacts are canonical Preserves evidence too. Failures emit `<harness-failure-v1 "molten.harness.failure.v1" ...>` on stdout by default, or write it to `--report-out`/`--failure-out` when supplied, and still exit non-zero:

```sh
cargo run -- test run bad-suite.preserves --report-out target/molten-reports/bad.failure.preserves
cargo run -- test report show target/molten-reports/bad.failure.preserves
cargo run -- test replay target/molten-reports/tampered.report.preserves --failure-out target/molten-reports/replay.failure.preserves
cargo run -- test report validate target/molten-reports/tampered.report.preserves --failure-out target/molten-reports/validate.failure.preserves
cargo run -- test repro export target/molten-reports/tampered.report.preserves --out target/molten-repro/tampered --failure-out target/molten-reports/export.failure.preserves
cargo run -- test gate check target/molten-reports/bad.failure.preserves --failure-out target/molten-reports/gate.failure.preserves
```

