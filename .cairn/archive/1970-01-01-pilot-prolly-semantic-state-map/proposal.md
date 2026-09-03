## Why

Molten world commits identify a typed semantic-state root, but the current roadmap does not select its ordered persistent-map mechanism.

A Prolly tree can provide history-independent structure, content-addressed immutable nodes, structural sharing, and subtree-skipping diffs. These properties fit branchable semantic state. They do not fit every world root, and they do not transfer branch-head, authority, effect, snapshot, or retention policy into the map.

The storage profile also needs stronger bounds than a copied reference algorithm. Canonical root identity depends on key encoding, comparison, node codec, split rules, size accounting, hash domains, and format version. Public split rules also need forced bounds against chosen keys and variable-size values.

## What Changes

- Add a Molten-local Prolly semantic-state map behind an application-owned block-store port.
- Define canonical leaf and internal nodes with domain-separated BLAKE3 identities.
- Bind key encoding, ordering, node codec, boundary seed, size accounting, split profile, and format version into one typed profile identity.
- Require history-independent roots for equal canonical maps, independent of admitted mutation order.
- Use key-derived, size-aware boundaries with named minimum, target, and forced maximum byte bounds.
- Add pure get, range, edit, diff, closure, and GC-planning logic. Keep block I/O, CAS, transactions, and deletion in shell adapters.
- Add Trellis proof obligations and property tests for ordering, boundary determinism, edit locality, diff soundness, reachability, and branch isolation.
- Compare bounded behavior with the pinned DoltLite oracle and the existing world sharing and retention benchmark.
- Keep the implementation Molten-local until evidence identifies a second credible consumer with the same contract.

## Dependencies

- `introduce-world-commit-core`.
- `add-world-state-diff-and-merge`.
- `add-world-commit-replication-and-retention`.
- `harden-world-commit-crash-and-concurrency`.
- `benchmark-world-commit-sharing-and-retention`.
- `pilot-doltlite-semantic-state-oracle` for optional differential evidence.
- Published Content Identity Core framing and Schema Migration Core planning contracts.

## Non-Goals

- A universal representation for task, effect, authority, snapshot, or executable-extent roots.
- Branch-head arbitration, authority checks, effect release, replication policy, or destructive GC execution inside the map core.
- Reusing transfer chunk geometry as tree-node boundary policy.
- Claiming collision impossibility, asymptotic performance without measurements, or general database correctness.
- Creating a shared repository before a second matching consumer and benchmark evidence exist.

## Impact

- **Core**: canonical nodes, profile identity, search, edit, diff, closure, reachability, and GC plans.
- **Ports**: narrow immutable block read and staged block publication capabilities.
- **Shell**: local block-store adapter, transaction orchestration, CAS, crash recovery, retention admission, and deletion.
- **Configuration**: typed Nickel map and benchmark profiles with named bounds.
- **Testing**: positive round trips and structural sharing plus negative malformed node, bad ordering, adversarial split, wrong profile, missing child, stale block, collision-assumption, and incomplete reachability cases.
