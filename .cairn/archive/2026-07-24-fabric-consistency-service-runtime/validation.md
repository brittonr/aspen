## Validation Evidence

- `cargo nextest run --profile deterministic` passed 1,242 tests with 103 profile skips (run `48512122-8647-4b4e-8ff4-9f941e508bc5`).
- `cargo nextest run --profile distributed-simulation` passed 67 tests with 1,278 profile skips (run `24ad4add-5706-4954-8d03-e4c18bf1914a`).
- The focused `fabric_consistency` library rail passed 63 tests. It covers exact startup and extension isolation, elections, replication, commit, read-index currentness, quorum loss, durable recovery, snapshots, fencing, resource bounds, cancellation, cleanup, evidence selection, operator preflights, model-profile production denial, and the three-process fixture.
- The three-process fixture passed with distinct process IDs, endpoint identities, and durable roots; majority commit/read, deliberate follower lag, durable snapshot catch-up, protocol partition, quorum loss, crash of every process, restart in new process IDs, term-2 recovery, stale term-1 leader rejection, and clean drain all remained within admitted ports.
- Selected commit and read-currentness evidence now binds a canonical quorum witness containing the exact static membership, configuration epoch, term, index, source effect ref, and sorted distinct acknowledgement members. Offline validation passes an admitted majority and denies duplicate acknowledgements, a minority, an outsider, and a tampered distinct-process receipt before accepting its quorum ref.
- `cargo clippy -p molten --all-targets -- -D warnings`, `cargo fmt --all --check`, and the repository pre-commit checks passed for the final core change.
- `cargo octet check` completed with zero errors, but the repository-wide strict Octet source gate remains denied. Receipt `blake3:5aed28b03ebd23890e803e026aa5502ed64a498a923549cfc97522682738587a` records `warning-only`, 5,576 findings, 230 unreviewed critical findings, complete artifact bindings, and failed `strict-status-clean`/`no-critical-findings` checks. A current sibling-runner all-target probe also fails closed on the pre-existing capability locator `tests/../src/test/support.rs`; its lib-only probe exits zero with warning-only findings. These repository-wide results are not promoted into a clean source-gate claim.
- Cairn strict validation and the proposal, design, and tasks gates passed before lifecycle closure. They are rerun after task completion and again after sync/archive.

## Admission Boundary

The live profile remains `production_admitted: false`. This change establishes the bounded implementation and declared distributed fault evidence, but no separate environment-scoped production policy/operator approval receipt exists, and the strict Octet source gate is not clean. Archiving this change therefore does not select the live profile as a production default or claim release, deployment, WAN, Byzantine, dynamic-membership, lease-read, leadership-transfer, cross-group transaction, or whole-system readiness.

## Evidence Boundaries

Passing process fixtures establish observations only for the declared three-voter static profile, admitted Iroh/Redb/Tokio/application/supervision cohort, bounded test resources, and injected failure plan. Canonical quorum validation proves structural membership, uniqueness, majority, and binding consistency for supplied evidence; it does not independently prove transport provenance, host isolation, authorization, source correctness, arbitrary schedules, or production approval.
