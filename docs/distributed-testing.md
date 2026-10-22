# Distributed testing evidence

Molten distributed testing is layered so developers can run the cheapest useful check first and reviewers can tell simulation evidence from VM or live evidence.

## Deterministic simulation fault plans

The pure simulation layer models peers, channels, virtual time, workflow commands, and explicit fault events. It canonicalizes these inputs into refs for:

- distributed topology;
- scheduler profile;
- deterministic seed;
- fault plan;
- distributed test run receipt.

Fault plans cover delay, drop, duplicate, reorder, partition, rejoin, crash, restart, stale evidence, ambient-state drift, unauthorized transport evidence, corrupted receipts, and resource pressure. Faults target explicit peers, channels, operations, or virtual time windows; host paths, wall-clock time, process ids, real sockets, and ambient randomness are not inputs.

Simulation receipts are regression and review evidence only. They do not grant authority, policy, provenance, resource, source-gate, retention, transport, destructive-operation, deployment, or production-readiness trust.

Named direct fixtures cover benign `delay`, `drop`, `reorder`, `rejoin`, `crash`, `restart`, and `duplicate` suppression paths, plus deny-before-side-effects coverage for `stale-evidence`, `corrupted-receipt`, `resource-pressure`, `unauthorized-transport`, `ambient-state-drift`, and `partition` quorum faults. Generated-case fixtures bind seed, topology, scheduler, command sequence, fault plan, invariant name, run ref, replay ref, event refs, final-state ref, and diagnostics in `generated-distributed-repro-v1`; these artifacts are diagnostic-only unless a separate pass/deny gate accepts the claim. The curated composite regression suite binds named cases for duplicate-after-restart, partition-with-stale-evidence, reorder-with-reconcile, crash-during-dispatch, and resource-pressure-during-quorum in `composite-fault-suite-v1`. Generated failures require promotion metadata with stable refs, profile eligibility, traceability, retry policy, variance declarations, and cost budget before becoming release-review claims. The focused command is `cargo test --lib distributed`.

## Declarative multinode scenario fixtures

Multinode scenarios are declared with typed Nickel fixtures under `docs/multinode-scenario-fixtures/` and the shared contract in `docs/multinode-scenario-contracts.ncl`. The contract requires explicit topology/profile ids, command surface, artifact kinds, topology/seed/fault-plan refs, receipt refs, variance refs, diagnostic-log refs, unavailable policy, and evidence-only caveats before execution. Valid VM fixture coverage includes smoke, live-control, service/job, restart, fault, three-node quorum, and aggregate scenario surfaces.

Rust validation derives `multinode-scenario-metadata-v1` from explicit fixture values only. Missing topology, missing command surface, stale or malformed refs, undeclared variance, unsupported pass claims, and mismatched artifact kinds deny before pass metadata is accepted.

## Topology profiles and reconciliation

The topology-profile matrix names pairwise transport, control quorum, restart/rejoin, subscriber peer, three-node quorum, and wrong-topology negative profiles. Each profile binds role membership, allowed links, required receipt kinds, and evidence scope. The three-node profile is a bounded platform-integration shape for voter, restarting-member, and subscriber/observer evidence; it is not fleet-scale or WAN evidence. A subscriber cannot satisfy voter membership, and transport-only evidence cannot satisfy authority, policy, quorum, or membership evidence.

`multinode-reconciliation-gate-v1` compares per-node summaries against the declared topology, scenario fixture, required receipts, equality classes, and allowed variance refs. Queue, ledger, dispatch, ack, protocol, chunk, or receipt-index refs must match unless a visible variance ref permits the difference. Missing node evidence, stale refs, wrong topology, duplicate semantic commits, divergent queues, and log-only reconciliation deny.

## Service recovery composition

The deterministic track composes three services through the typed facades and one recovery-group declaration: a durable
stateful service, a dependent worker ordered after it in the group, and an independent sibling outside the group. One
scenario per failure class records what the composition observed, and the property evaluation turns those observations
into five named properties: unaffected services continue, stale instances lose influence, durable work is readmitted
under a current claim, uncertain external effects are not blindly repeated, and restart storms reach a terminal outcome
inside the admitted window.

The scenarios are the member failure with the dependent restart, the delayed pre-restart event delivery, the durable
work readmission, the uncertain effect surfacing, the queue overload, the restart storm to escalation, and the generation
upgrade with stale-work rejection. Each scenario asserts every property that applies to it, so a regression in one
mechanism fails several scenarios.

Two deliberately broken variants keep the property assertions honest. The fencing-removed variant restarts without
advancing the incarnation, and the group-budget-reset variant clears the restart history on every replacement. The
harness must fail the matching property for each; a passing broken variant would mean the property assertion is
vacuous. A failing property is evidence against the mechanism that owns it, not a patch inside the composition.

The CPU-saturation case under the time-sliced execution profile and the real-process twin are not part of this
composition yet: the first needs the admitted time-sliced profile, and the second needs the outside-in process-fault
track and its observation conventions.

## Local multiprocess harness

The local multiprocess harness model has a pure planner that validates node identities, state-root handles, transport handles, command-plan refs, expected receipt refs, and cleanup policy. The shell may spawn processes and collect artifacts, but the plan denies state-root collisions, transport collisions, stale tickets, missing receipts, child timeouts, orphaned children, or missing cleanup before accepting local pass evidence. `local-multiprocess-executable-run-v1` binds the shell observations to the pure plan and local run receipt. Local multiprocess receipts are local integration evidence only; they do not replace NixOS VM, WAN/live, authority, policy, provenance, resource, source-gate, retention, deployment, or production-readiness evidence.

## Outside-in fault track

r[impl molten.outside_in.observation] The outside-in tier (`tests/outside_in/`, nextest profile `outside-in`) runs ordinary node processes through public entry points over the real Iroh transport and a real local Redb store. The receiver is a node state root driven by `molten node init`, `run`, `live-peer-admit`, and `serve --live-iroh`. The client is a separate `molten node control-ingress-live-send` process. The scenario reads histories through `molten node control-history`. The live control path's Redb store is the receiver's delivery idempotency store, `control/idempotency/delivery-idempotency.redb`.

Each scenario declares its process, wall-clock, retained-output, and client-operation budgets before it starts (`molten_core::outside_in`). The harness owns every process and scratch path it creates and tears them down. The scenarios cover five profiles:

- **No fault:** one request is delivered and processed once.
- **Kill and restart:** at the `listener-awaiting-request` boundary, a listener waiting after a committed request is killed and restarted on the same storage. The committed request must still appear in the post-restart history, and every run must report the same bound endpoint.
- **Pause and resume:** at the same boundary, the listener is stopped with `SIGSTOP` while a client sends, then resumed.
- **Partition and heal:** at the `transport-between-peers` boundary, the client dials a loopback UDP forwarder that carries the real Iroh QUIC datagrams. It drops every datagram during a bounded hold, so the partitioned attempt is denied. After the heal, the retry is delivered once.
- **Lost response:** at the `client-response` boundary, the harness drops the client's response and records the outcome as unresolved. A retry with the same sequence is suppressed by the receiver's idempotency store.

Judgment uses only observed outcomes: client attempts, the per-envelope delivery outcomes, pending requests, and processed requests. No observation carries a time. A result is agreement; a permitted divergence with its reasons (lost response applied, retried after fault, duplicate suppressed); a failure naming the first mismatch; or a typed unsupported, incomplete, startup-failed, budget-exhausted, or rejected-declaration outcome. A scenario passes only if its teardown released everything it owned. Negative fixtures cover a startup failure, each budget exhausted, a fault the host cannot apply, an unreadable history, and a retained scratch path.

The consensus path is excluded: a consensus-path declaration is rejected before any process starts, and the ChaosControl and live consensus campaigns own it. The track complements deterministic simulation and does not replace it: simulation explores fault interleavings deterministically, while this tier exercises real processes, sockets, signals, and stores at a few declared boundaries. Results carry the no-production-admission, no-simulation-replacement, and no-performance non-claims. They are local integration evidence, not VM, WAN, production, or performance evidence.

## Cluster lifecycle receipts and drift summaries

Cluster lifecycle tests can produce `cluster-lifecycle-run-v1` review receipts over explicit observations from `cluster init`, `start`, `status`, and `stop` workflows. The receipt binds workflow id, manifest ref, ordered node ids, phase decisions, per-node config/identity/startup/health/control/heartbeat/shutdown refs, already-running observations, reverse stop order, diagnostics, and evidence-only caveats. Missing phase receipts, duplicate node summaries, stale manifest refs, wrong stop order, invalid refs, missing required per-phase refs, or stdout-only success deny before pass evidence.

Drift comparison uses a receipt-derived summary rather than rendered terminal output. Summary fields include manifest ref, node order, stop order, phase decisions, per-node lifecycle refs, already-running refs, and caveats; temporary roots, diagnostic logs, and other variance must be declared as explicit drift variance fields. Changed child refs, node ordering drift, field-kind drift, stale variance declarations, ambient state, retry-only success, or rendered-output-only stability remain denial evidence.

## Failure repro bundles

Sealed multinode failure repro bundles bind scenario fixture refs, topology refs, scheduler refs, seed refs, fault-plan refs, command refs, node-summary refs, receipt refs, diagnostics, log refs, redaction policy refs, replay status, private attachment refs, reveal receipts, and evidence-only caveats. Simulation bundles may replay deterministically; local multiprocess and VM bundles verify as non-replayable diagnostic evidence unless a separate recorded effect log exists. `vm-failure-repro-export-v1` is emitted only for denied, unavailable, or failed-validation VM evidence and remains diagnostic-only. Tampered, unsealed, stale, private-without-reveal, missing-redaction, or diagnostic-only bundles fail closed before private content is materialized or before any pass claim is accepted.

## What reviewers inspect

A `distributed-test-run-v1` receipt binds the source ref, test binary ref, topology ref, seed ref, scheduler profile ref, fault-plan ref, child workflow refs, emitted event refs, final state ref, replay status, allowed variance refs, diagnostics, and pass or deny decision.

A denial identifies the invariant that failed before side effects, such as missing authority, transport evidence being treated as authority, stale evidence, partitioned quorum, corrupted receipts, or undeclared ambient state.

## Distributed CI risk matrix

Release review uses an explicit risk/cost matrix:

| Profile | Command surface | Evidence scope |
| --- | --- | --- |
| `fast-core` | `cargo nextest run --profile fast-core` | pure core/unit/parser/receipt checks; no platform or transport claims |
| `harness` | `cargo nextest run --profile harness` | report, replay, gate, repro, redaction, and failure artifact behavior |
| `cli` | `cargo nextest run --profile cli` | command integration and receipt writing; stdout/stderr are views only |
| `distributed-simulation` | `cargo nextest run --profile distributed-simulation` or `cargo test --lib distributed` | deterministic simulation and model invariants |
| `vm-platform` | `nix build .#checks.x86_64-linux.nixos-vm-multinode` | two-node NixOS platform and executable VM fault evidence when host support is available |
| `dogfood-soak` | `nix build .#checks.x86_64-linux.dogfood-local-node` | pilot/readiness review only |

Every distributed shard binds metadata for source/tree refs, Nix input refs, test binary/package refs, profile, shard, seed, topology, fault plan, emitted receipts, variance declarations, and diagnostic log refs. Diagnostic logs stay diagnostic-only; canonical receipts and metadata refs are the review surface.

Traceability coverage is required for distributed evidence-bearing requirements. A release or CI gate must include positive evidence and negative evidence, or an explicit exemption. Direct simulation fixture traceability now names positive and negative coverage for `molten.testing.distributed_simulation.direct_fault_fixtures`, `molten.testing.distributed_simulation.fixture_traceability`, and `molten.testing.distributed_ci.profile_wiring_evidence`. Missing positive coverage, missing negative coverage, stale refs, retry-only success, skipped VM support, or undeclared variance denies the distributed evidence gate.

CI/release pass evidence uses zero retries. Exploratory reruns may produce diagnostic or quarantine evidence, but a pass after retry is not deterministic pass evidence unless a separate review artifact accepts the remediation boundary.

Unsupported VM, network, live transport, or soak support records `unavailable`, `skipped`, or `deny` evidence. Network-control support is probed with `nixos-vm-network-control-probe-v1` before executable delay, drop, partition, rejoin, or asymmetric-latency fault claims. Unsupported execution never counts as pass evidence for a claim that requires that profile.

## Smallest useful check

Use simulation when changing distributed protocol logic, idempotency, replay, authority separation, or restart behavior and you need fast deterministic feedback before VM checks.

Use NixOS VM checks when the claim depends on platform integration, systemd, filesystem state roots, QEMU networking, service restart behavior, or true cross-node live transport. Shard receipts (`nixos-vm-shard-run-v1`) localize smoke, live-control, service/job, restart, and fault failures and carry an explicit evidence scope: `fixture-metadata` for synthetic planning/readback, `executable-vm` for real VM child receipts, `aggregate-index` for index-only receipts, and `diagnostic-only` for unavailable or log evidence. The aggregate receipt (`nixos-vm-multinode-aggregate-v1`) preserves child scopes without converting fixture metadata, unavailable, denied, stale, or log-only evidence into executable platform pass evidence. Live transport VM evidence must bind send, receive, ingress, queue, dispatch, reconcile, ack, and protocol-gate receipts before artifact export; logs cannot replace a receive receipt. Use live soak/pilot evidence only for scoped operator-readiness review.
