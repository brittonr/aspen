## Context

`aspen-kv-branch` wraps any `KeyValueStore` and buffers writes in a copy-on-write overlay. Its `commit-dag` feature records commits and branch tips through `aspen-commit-dag`. The branch crate is already mostly generic, but `aspen-commit-dag` currently imports `ChainHash`, `GENESIS_HASH`, `hash_from_hex`, `hash_to_hex`, and `constant_time_compare` from `aspen_raft::verified`. That one dependency pulls the family back into the Raft compatibility shell and blocks a clean extraction story.

The family has existing specs for branch commit behavior and commit DAG semantics. This change does not redesign branch semantics; it hardens the crate boundary and evidence around those APIs.

## Goals / Non-Goals

**Goals:**

- Make `aspen-commit-dag` independent from `aspen-raft` and root Aspen app/runtime shells.
- Preserve `aspen-kv-branch` default behavior and the named `commit-dag` feature.
- Prove both crates compile as reusable libraries with dependency-boundary evidence.
- Prove representative Aspen consumers still compile through existing feature names.
- Document the extraction contract and readiness state.

**Non-Goals:**

- Publishing to crates.io or splitting repositories.
- Renaming crates.
- Changing commit hash semantics, branch commit semantics, fork semantics, or GC semantics.
- Extracting downstream consumers such as jobs, deploy, FUSE, docs, or CLI.
- Moving the whole `aspen-raft::verified` module; only the branch/DAG-owned hash helpers required by this family are in scope.

## Decisions

### Decision 1: Localize chain hash helpers to the branch/DAG family

**Choice:** Move or duplicate the minimal chain hash helper surface used by `aspen-commit-dag` into `aspen-commit-dag::verified` unless implementation reveals another reusable leaf crate is already warranted.

**Rationale:** The helpers are small, pure, deterministic BLAKE3/hex utilities. Creating a new workspace crate is only justified if more than this family needs the extracted surface immediately.

**Alternative:** Keep depending on `aspen-raft::verified`. Rejected because that defeats the extraction boundary and keeps the family coupled to a compatibility shell.

### Decision 2: Keep `commit-dag` as an opt-in feature on `aspen-kv-branch`

**Choice:** `aspen-kv-branch` default features stay branch-overlay-only. The commit DAG integration remains behind the named `commit-dag` feature.

**Rationale:** Branch overlays are useful without persisted commit history. Keeping the feature boundary lets downstream users choose the lighter branch overlay or the full branch+DAG integration.

**Alternative:** Make commit DAG a default dependency. Rejected because it widens the default graph and weakens the extraction boundary.

### Decision 3: Prove extraction through an in-tree downstream fixture

**Choice:** Add a fixture that depends on the canonical crates directly and uses a deterministic `KeyValueStore`-style test adapter or minimal local adapter to compile branch and commit DAG APIs without root Aspen or compatibility re-exports.

**Rationale:** This matches the Redb Raft KV and coordination extraction pattern while staying maintainable in the monorepo.

**Alternative:** Use only workspace `cargo check` commands. Rejected because workspace feature unification can hide accidental dependencies.

## Risks / Trade-offs

- **[Risk] Hash helper drift** → Mitigate with tests that compare old and new commit IDs for representative inputs before removing the Raft dependency.
- **[Risk] Feature unification hides leaks** → Mitigate with `cargo tree` and fixture metadata from isolated manifests.
- **[Risk] Consumer features are stale** → Mitigate by compiling each named reverse consumer with the feature that reaches the branch/DAG family.
- **[Trade-off] No new leaf hash crate initially** → Acceptable because localizing first is simpler; a later crate can be introduced if more families need the same helper surface.
