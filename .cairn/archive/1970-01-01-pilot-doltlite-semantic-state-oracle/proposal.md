## Why

Molten owns immutable world commits, branch heads, typed merge policy, and replay. It does not yet have an independent storage oracle for branchable semantic state.

A native map can pass its own tests while sharing one encoding, mutation, or concurrency defect across implementation and oracle. DoltLite offers a separate SQLite-facing implementation with content-addressed Prolly trees, branch-local working state, snapshot rules, CAS-style head movement, GC coordination, and an unusually explicit compatibility ledger.

DoltLite also has boundaries that Molten must not inherit. Its current format is beta and exact-versioned. It rejects writes across multiple file-backed databases, keeps conflicts transaction-local, uses backend-local identities, and does not own Molten authority or effect release.

## What Changes

- Add an optional, test-only DoltLite semantic-state adapter pinned to commit `10170ed82c1b12414db8d1b29d2fe9ea2a72fd88`.
- Build the adapter with remotes disabled and run it only beneath disposable capability-rooted test directories.
- Map canonical Molten semantic keys and values through explicit primary keys without SQLite rowid identity.
- Add a typed Nickel compatibility ledger with closed statuses, evidence links, negative fixtures, and a non-increasing exception ratchet.
- Exercise history independence, detached reads, stale-snapshot denial, branch isolation, compare-and-advance races, reader-safe GC, exact format rejection, and serialization behavior.
- Record intentional differences for complete-world atomicity, durable conflicts, authority, effects, and stack-global BLAKE3 identities.
- Publish normalized semantic observations for the Molten benchmark and differential rails without treating agreement as correctness proof.

## Dependencies

- `introduce-world-commit-core`.
- `add-world-branch-head-protocol`.
- `add-world-state-diff-and-merge`.
- `harden-world-commit-crash-and-concurrency`.
- `benchmark-world-commit-sharing-and-retention`.

## Non-Goals

- A production Molten database or default runtime dependency.
- SQLite or Dolt SQL compatibility as a product promise.
- DoltLite remotes, network authority, or replication authority.
- Reusing DoltLite hashes as Molten world or semantic-state identities.
- Delegating complete-world atomicity, durable conflicts, merge policy, head policy, effect release, or retention authority to DoltLite.
- Copying third-party source outside its applicable license and notice requirements.

## Impact

- **Test shell**: optional DoltLite build, disposable database lifecycle, normalized observation adapter, and bounded command execution.
- **Contracts**: Nickel compatibility rows, closed status vocabulary, evidence requirements, and exception ratchet.
- **Evidence**: source revision, build identity, backend format, exact test profile, normalized observations, and explicit non-claims.
- **Testing**: positive compatible behavior plus negative divergence, malformed format, stale writer, GC race, unsupported operation, and overclaim cases.
