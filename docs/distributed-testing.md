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

## What reviewers inspect

A `distributed-test-run-v1` receipt binds the source ref, test binary ref, topology ref, seed ref, scheduler profile ref, fault-plan ref, child workflow refs, emitted event refs, final state ref, replay status, allowed variance refs, diagnostics, and pass or deny decision.

A denial identifies the invariant that failed before side effects, such as missing authority, transport evidence being treated as authority, stale evidence, partitioned quorum, corrupted receipts, or undeclared ambient state.

## Smallest useful check

Use simulation when changing distributed protocol logic, idempotency, replay, authority separation, or restart behavior and you need fast deterministic feedback before VM checks.

Use NixOS VM checks when the claim depends on platform integration, systemd, filesystem state roots, QEMU networking, or service restart behavior. Use live soak/pilot evidence only for scoped operator-readiness review.
