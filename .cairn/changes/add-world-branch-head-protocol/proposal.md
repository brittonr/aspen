## Why

The world-commit core will provide immutable causal snapshots. Molten still needs durable names for active and candidate branches.

Choregraph supplies pure branch-reference and compare-and-swap planning. Artifact Auth supplies canonical signed statements. Neither owns Molten branch policy, head storage, activation, or conflict handling.

A signature alone cannot reject an old claim against intact current state. Molten needs generation-fenced head claims, explicit currentness, and fail-closed handling for competing writers. Local generations cannot detect rollback of both the head and its generation store.

## What Changes

- Add canonical world-branch identities and detached `world-head-claim-v1` statements.
- Bind each claim to an expected head, successor commit, head generation, purpose, branch policy, and exact signer observations.
- Require the successor to descend from the expected head under declared branch or merge rules.
- Use Choregraph branch planning for pure generation-fenced compare-and-swap decisions.
- Use Artifact Auth for bounded statement authentication without transferring authorization or signing authority.
- Persist head changes atomically in the Molten shell after current policy and authority admission.
- Preserve competing valid claims as explicit conflicts. Do not use timestamps or last-writer-wins selection.
- Classify generation checks as stale-claim and replay protection under intact durable state, not external rollback proof.
- Add inspection, conflict, stale-claim, and currentness diagnostics with detached evidence.

## Dependencies

- `introduce-world-commit-core`.
- Choregraph branchable event history.
- Artifact Auth canonical statement and Ed25519 verification packages.
- Durable Authority State for currentness, replay, and revocation observations.

## Non-Goals

- Distributed consensus, automatic conflict resolution, or proof of remote convergence.
- Private-key storage, signer discovery, global trust, or authorization inside Artifact Auth.
- Effect release, state merge, content replication, or garbage collection.
- Detection of whole-store rollback without an independent currentness or witness observation.

## Impact

- **Core**: branch IDs, head claims, ancestry checks, generation fences, conflict sets, and mutation plans.
- **Shell**: current head storage, signing adapters, authentication inputs, authority rechecks, and atomic compare-and-swap publication.
- **Schemas**: canonical Preserves head claims, conflict reports, and transition receipts.
- **Testing**: valid advances plus negative stale heads, old generations, whole-store rollback overclaims, missing ancestry, threshold failures, wrong purpose, stale authority, and concurrent claims.
