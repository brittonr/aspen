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

Named direct fixtures cover benign `delay`, `drop`, `reorder`, `rejoin`, `crash`, `restart`, and `duplicate` suppression paths, plus deny-before-side-effects coverage for `stale-evidence`, `corrupted-receipt`, `resource-pressure`, `unauthorized-transport`, `ambient-state-drift`, and `partition` quorum faults. The focused command is `cargo test --lib distributed`.

## What reviewers inspect

A `distributed-test-run-v1` receipt binds the source ref, test binary ref, topology ref, seed ref, scheduler profile ref, fault-plan ref, child workflow refs, emitted event refs, final state ref, replay status, allowed variance refs, diagnostics, and pass or deny decision.

A denial identifies the invariant that failed before side effects, such as missing authority, transport evidence being treated as authority, stale evidence, partitioned quorum, corrupted receipts, or undeclared ambient state.

## Distributed CI risk matrix

Release review uses an explicit risk/cost matrix:

| Profile | Command surface | Evidence scope |
| --- | --- | --- |
| `fast` | `cargo nextest run --profile deterministic` | pure core/unit/parser/receipt checks; no platform or transport claims |
| `protocol` | `cargo test --lib distributed` | deterministic simulation and model invariants |
| `cli` | `nix build .#checks.x86_64-linux.requirement-traceability-gate` | local CLI receipt and traceability behavior |
| `vm-smoke` | `nix build .#checks.x86_64-linux.nixos-vm-multinode` | two-node NixOS platform smoke evidence when host support is available |
| `vm-fault` | `nix build .#checks.x86_64-linux.nixos-vm-multinode` | bounded executable VM fault evidence when host support is available |
| `soak` | `nix build .#checks.x86_64-linux.dogfood-local-node` | pilot/readiness review only |

Every distributed shard binds metadata for source/tree refs, Nix input refs, test binary/package refs, profile, shard, seed, topology, fault plan, emitted receipts, variance declarations, and diagnostic log refs. Diagnostic logs stay diagnostic-only; canonical receipts and metadata refs are the review surface.

Traceability coverage is required for distributed evidence-bearing requirements. A release or CI gate must include positive evidence and negative evidence, or an explicit exemption. Direct simulation fixture traceability now names positive and negative coverage for `molten.testing.distributed_simulation.direct_fault_fixtures`, `molten.testing.distributed_simulation.fixture_traceability`, and `molten.testing.distributed_ci.profile_wiring_evidence`. Missing positive coverage, missing negative coverage, stale refs, retry-only success, skipped VM support, or undeclared variance denies the distributed evidence gate.

CI/release pass evidence uses zero retries. Exploratory reruns may produce diagnostic or quarantine evidence, but a pass after retry is not deterministic pass evidence unless a separate review artifact accepts the remediation boundary.

Unsupported VM, network, live transport, or soak support records `unavailable`, `skipped`, or `deny` evidence. Unsupported execution never counts as pass evidence for a claim that requires that profile.

## Smallest useful check

Use simulation when changing distributed protocol logic, idempotency, replay, authority separation, or restart behavior and you need fast deterministic feedback before VM checks.

Use NixOS VM checks when the claim depends on platform integration, systemd, filesystem state roots, QEMU networking, or service restart behavior. Use live soak/pilot evidence only for scoped operator-readiness review.
