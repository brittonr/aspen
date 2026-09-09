# Design: Reduce real storage work through structural sharing

## Context

Structural sharing today saves storage, not computation. The review identified three
concrete sites: full-rebuild edits, post-hoc `skipped_equal_nodes`, and per-block
read transactions. The dependency `harden-prolly-publication-integrity` lands first;
this change assumes commit classification and GC currentness are already sound.

## Goals

- Convert existing structural sharing into avoided reads, decodes, and transactions.
- Preserve exact canonical outputs; any deviation requires a new named profile.
- Keep Redb types out of `molten-core`.

## Non-Goals

- No unconditional logarithmic update bound: content-defined boundaries can shift,
  so incremental edits bound work and retain a rebuild fallback instead.
- No page-cache imports, lock-free trees, or profile-limit changes.
- No fast-path equality check that bypasses required validation.

## Approach

### Measurements first

Extend the Prolly service observations with pure counters: `nodes_read`,
`nodes_decoded`, `subtrees_not_traversed`, `transactions_opened`, and `bytes_copied`.
Establish a baseline within the 4,096-entry standard profile before and after each
optimization. Split the current `skipped_equal_nodes` into `shared_node_count`
(structural fact) and actual avoided-traversal counts (behavioral fact).

### Bounded read sessions

Add a read-session operation to `ProllyBlockStorePort`:

```
open_read_session() -> ReadSession
read_block(session, node_ref) -> bounded request
close_read_session(session) -> observation
```

The Redb adapter holds one `begin_read` transaction per session; the pure core sees
only the port requests and a closing observation. Sessions carry explicit lifetime
and resource bounds. Operations that may wait on network retention use a canonical
retention pin rather than holding a local transaction.

### Genuine subtree-skipping diff

The diff planner compares admitted roots, then child identities and key ranges:
matching validated subtrees are skipped, and blocks are requested only for differing
regions. Validation contracts stay separate: a verified path is not a fully validated
closure, and a root-string match never admits an untrusted snapshot. Acceptance is
exact diff-record equality with the current implementation plus demonstrated
reductions in `nodes_read` and `nodes_decoded`.

### Incremental edits

`incremental_edit` reconstructs only affected regions and reuses the rest, continuing
to rebuild until content-defined boundaries resynchronize. Acceptance is exact
canonical-root equality against `full_rebuild(apply(existing_entries, edits))` for
every case, including adversarial boundary shifts, at tiny capacities where eviction
and rebuild paths stay reachable. Work limits trigger the rebuild fallback. The
existing multi-edit interface is exercised first: applications pass one atomic
transition's edits together without merging distinct semantic transactions.

### Decoded-node cache

Extend the dataspace-access cache principles: explicit capacity, deterministic
retention, loader outside the mutex, deferred destruction, and no freshness claim on
hit. Cache identity is (canonical node reference, profile, codec/validation version).
Retention is advisory: changing it must not change any canonical output, retry
behavior, ordering, admission, or emitted evidence, preserving deterministic
playback. Cached node contents are immutable; no entry is mutated under its old
content reference.

### Reuse of komora-io code

Where a komora-io component matches (fault injection dev-dependency, cache
primitives), reuse it with recorded provenance and license rather than copying
source; GPL and unlicensed komora crates remain excluded per the recorded adoption
audit.

## Testing

- Reference oracles: the retained rebuild-first planner and an independent ordered-map
  model; compare exact canonical roots and exact diff records, not entry counts.
- Differential and property tests at tiny capacities (1–4 entries per node) to force
  boundary shifts, eviction, and fallback paths.
- Positive and negative cases for every port change, including session misuse
  (use-after-close, bound exhaustion) and cache identity mismatches.

## Risks

- Incremental planners with shifting boundaries can silently diverge; the exact-root
  oracle and differential harness are the control.
- Read sessions add lifetime complexity; bounds and misuse negatives are required,
  not optional.
