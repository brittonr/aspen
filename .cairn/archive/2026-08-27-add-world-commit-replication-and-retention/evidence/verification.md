# World distribution verification

Date: 2026-08-27

## Completion scope

This evidence covers the focused Molten world-distribution change. It does not establish repository-wide release eligibility.

## Baseline

Before implementation, the current canonical base passed these focused tests:

- Eight generic DAG-sync core tests.
- Nine content-replication core tests.
- Twenty-seven world-commit, branch-head, merge, and related world tests.
- Existing retention checks remained green.

## Implemented boundary

The change adds:

- canonical world-commit and typed-root DAG projection;
- bounded missing-closure and resume planning through generic DAG sync;
- immutable protected-form transfer plans through generic content replication;
- a content-replication-to-DAG bridge with verify-before-progress behavior;
- separate authenticated head-claim exchange with current local admission;
- explicit competing-claim preservation without arrival-order selection;
- complete closed retention-class observations;
- conservative active, uncertain, contradictory, unavailable, and cleared remote lease handling;
- direct Artifact Binding Core reachability and pin paths;
- report-only handoff into existing retention dry-run planning;
- canonical Preserves request, closure, claim, retention, and reachability records;
- bounded operator planning and inspection commands; and
- fail-closed standalone sync and resume commands.

## Deterministic checks

The following commands passed from the isolated worktree with local Nix builders:

```text
cargo test -p molten-core world_distribution --all-features
  7 passed

cargo test -p molten world_distribution --all-features
  5 passed

cargo test -p molten --bin molten world_distribution --all-features
  2 passed

cargo clippy -p molten-core -p molten --all-targets --all-features -- -D warnings
  passed

nix build .#checks.x86_64-linux.world-distribution-octet-deny-all --no-link -L
  Status: clean
  Findings: 0
  Warnings: 0
  Errors: 0
  Profile hash: b3:f0d2a35c8c6abf5cfa09013dbd357b50311fd671b5e871aced612ce6a934e3f7

nix build .#checks.x86_64-linux.world-distribution-dependency-identity --no-link -L
  passed

nix flake check --no-build
  all checks evaluated successfully
```

Strict Cairn validation plus proposal, design, and tasks gates passed. Cairn sync promoted all six requirement IDs into the accepted world-commit specification.

The full `molten-core` and `molten` package test commands passed. Full `nix flake check -L` reached the inherited `contract-export-drift-gate` and failed because `cairn-policy/default.ncl` does not reproduce its checked-in generated JSON. This branch does not change either policy file. Focused world-distribution Nix checks pass, so the repository-wide policy drift remains a separate bounded blocker rather than world-distribution evidence.

## Positive evidence

Positive tests cover complete local closure, stable planning, durable resume fencing, protected transfer, receipt-last completion, authenticated claims, explicit conflicts, complete retention inventories, Artifact Binding pin paths, legal holds, and existing retention-plan handoff.

## Negative evidence

Negative tests cover identity substitution, missing roots, cycles, partial closure, unsolicited replicas, corrupt content, denied claim authority, competing heads, missing execution observations, unavailable remote leases, and retention evidence that cannot grant deletion authority.

## Claim boundary

A complete local closure is not activation authority. A valid signature is not current branch authority. A conflict record does not select a head. Reachability is not retention or deletion authority. A local receipt does not prove permanent durability, peer trust, global convergence, application correctness, or release eligibility.
