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

Multinode scenarios are declared with typed Nickel fixtures under `docs/multinode-scenario-fixtures/` and the shared contract in `docs/multinode-scenario-contracts.ncl`. The contract requires explicit topology/profile ids, command surface, artifact kinds, topology/seed/fault-plan refs, receipt refs, variance refs, diagnostic-log refs, unavailable policy, and evidence-only caveats before execution.

Rust validation derives `multinode-scenario-metadata-v1` from explicit fixture values only. Missing topology, missing command surface, stale or malformed refs, undeclared variance, unsupported pass claims, and mismatched artifact kinds deny before pass metadata is accepted.

## Topology profiles and reconciliation

The topology-profile matrix names pairwise transport, control quorum, restart/rejoin, subscriber peer, three-node quorum, and wrong-topology negative profiles. Each profile binds role membership, allowed links, required receipt kinds, and evidence scope. The three-node profile is a bounded platform-integration shape for voter, restarting-member, and subscriber/observer evidence; it is not fleet-scale or WAN evidence. A subscriber cannot satisfy voter membership, and transport-only evidence cannot satisfy authority, policy, quorum, or membership evidence.

`multinode-reconciliation-gate-v1` compares per-node summaries against the declared topology, scenario fixture, required receipts, equality classes, and allowed variance refs. Queue, ledger, dispatch, ack, protocol, chunk, or receipt-index refs must match unless a visible variance ref permits the difference. Missing node evidence, stale refs, wrong topology, duplicate semantic commits, divergent queues, and log-only reconciliation deny.

## Local multiprocess harness

The local multiprocess harness model has a pure planner that validates node identities, state-root handles, transport handles, command-plan refs, expected receipt refs, and cleanup policy. The shell may spawn processes and collect artifacts, but the plan denies state-root collisions, transport collisions, stale tickets, missing receipts, child timeouts, orphaned children, or missing cleanup before accepting local pass evidence. `local-multiprocess-executable-run-v1` binds the shell observations to the pure plan and local run receipt. Local multiprocess receipts are local integration evidence only; they do not replace NixOS VM, WAN/live, authority, policy, provenance, resource, source-gate, retention, deployment, or production-readiness evidence.

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

Use NixOS VM checks when the claim depends on platform integration, systemd, filesystem state roots, QEMU networking, service restart behavior, or true cross-node live transport. Shard receipts (`nixos-vm-shard-run-v1`) localize smoke, live-control, service/job, restart, and fault failures; the aggregate receipt (`nixos-vm-multinode-aggregate-v1`) indexes child shards without converting unavailable, denied, stale, or log-only evidence into pass. Live transport VM evidence must bind send, receive, ingress, queue, dispatch, reconcile, ack, and protocol-gate receipts before artifact export; logs cannot replace a receive receipt. Use live soak/pilot evidence only for scoped operator-readiness review.
