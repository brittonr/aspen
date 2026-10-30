# Marble-plus-art object store spike: verification and decision

## Seam

The spike implements the existing Prolly `ProllyBlockStorePort` and
`ProllyRetentionPort` (`src/prolly_map/marble/`), behind the optional
`marble-object-store` Cargo feature. `config/marble-store/selection.ncl`
selects the backend: its generated JSON feeds `parse_prolly_store_selection`
and `open_prolly_store`. The reviewed selection is Redb, and its contract
(`ReviewedSelection`) refuses any other default. Without the feature, a marble
selection fails with `prolly-store-backend-unavailable`.

## Pure core (`crates/molten-core/src/marble_store/`)

The core owns:

- object-identifier ranges sharded by kind, and an allocator reseeded from the
  heap's high-water mark that fails the whole batch closed on exhaustion;
- the digest-framed block encoding;
- the canonical catalog encoding;
- batch planning and in-flight visibility, where mappings install only after a
  committed `write_batch` and a failed one poisons the instance;
- streaming index recovery, with duplicate and conflict detection;
- the maintenance schedule and a bounded read cache;
- the decision record's claim boundary (`validate_spike_decision`).

`cargo test -p molten-core --lib marble_store`: 16 passed.

## Shell

- `MarbleBackend` runs on one bounded `StoreExecutor` worker. The worker opens
  the heap, rebuilds the art index, and owns maintenance. A full queue is
  refused.
- Feature-gated Prolly tests: `cargo test -p molten --lib --features
  marble-object-store prolly_map` gives 26 passed (+1 ignored measurement
  harness). They cover:
  - a full lifecycle mirroring the Redb test: publish, collision, update,
    reopen, GC, and a reopen after GC;
  - absent and non-canonical digests;
  - identical-block dedupe;
  - object-identifier exhaustion, including across a reopen;
  - an interrupted batch, simulated by an unrenamed batch file, leaving no
    mapping after recovery;
  - a torn trailer refusing the open;
  - a failed write batch on a read-only heap poisoning the instance, after
    which a reopen recovers exactly the committed batches;
  - scheduled and explicit maintenance;
  - selection parsing, its refusals, and opening through the seam;
  - the recorded decision passing the core claim boundary, and a replace
    reading of it failing.
- Default features: `cargo test -p molten --lib prolly_map` passes, and the
  default path is unchanged.

## Pinning

The release-dependency profile moved to schema v2 with `registry_dependencies`
rows: transport `crates.io`, a `plane`, an exact version, and the Cargo.lock
checksum.

| Package | Version | Checksum | Plane | Disposition |
|---|---|---|---|---|
| `art` | 1.0.0 | `41e7ba0ded1ee3730fe4445fbdf4754756b2edd86617f19507c89d3b71383277` | implementation | optional-runtime |
| `marble` | 16.0.2 | `c00d04008994fed635a52323bbe1dd8c0947292183585ddf8f1090615fafc3c6` | implementation | optional-runtime |

`molten_core::release_dependency` validates each row:

- the manifest's exact `=` requirement;
- that the manifest table matches the plane;
- that the optional flag matches the disposition;
- the lock version, source, and checksum.

Its canonical material classifies both pins onto implementation edges. The
real profile validates against the repository (`decision=pass rows=21`).

Adding the two crates added 9 lock packages (`art`, `marble`, `concurrent-map`,
`ebr`, `fault-injection`, `fs2`, `pagetable`, `shared-local-state`,
`stack-map`), all MIT OR Apache-2.0. Cargo's resolve also moved three existing
edges to versions already in the lock:

- `data-encoding-macro-internal`: `syn` 1 → 2
- `snafu-derive`: `heck` 0.4 → 0.5
- `colored`: drops `windows-sys` 0.48

The same resolve without the new dependencies leaves the lock unchanged. Both
unit2nix plans were regenerated, and `scripts/git-source-hashes.sh --check`
passes. The plans resolve default features, so they do not contain marble.

## Measurements and decision

`config/marble-store/decision.ncl` records two workloads on one host. The run
used Linux 6.18.50, 32 threads, a ZFS dataset, and the unoptimized dev profile.
The harness is `prolly_map::tests::marble::measure`, run with `--ignored`.

| Workload | Metric | Redb | marble+art |
|---|---|---|---|
| 64 entries, 1 update | batch write | 2.01 ms | 0.88 ms |
| | point read | 7.0 µs | 12.5 µs |
| | recovery | 1.65 ms | 1.66 ms |
| | space amplification | 91.1x | 1.5x |
| 4096 entries, 32×16 updates | batch write | 6.95 ms | 6.33 ms |
| | point read | 6.5 µs | 28.1 µs |
| | recovery | 39.8 ms | 61.8 ms |
| | space amplification | 10.7x | 1.11x |

The verdict is **keep-current**.

- Point reads are 1.8x to 4.3x slower.
- Recovery is 1.55x slower at scale, because the art index is in memory only
  and every open reads every live object.
- Source review of marble 16.0.2 found:
  - undefined behavior over uninitialized memory in its read path
    (`uninit_boxed_slice`, `read_range_at`);
  - unchecked on-disk lengths sized before checksum verification;
  - `assert` and `unwrap` panics on inconsistent state;
  - runtime location-table corruption after a failed write, which requires
    instance poisoning;
  - ambient path-based heap authority.

Space amplification favors marble: Redb's file length includes preallocated
pages. This is recorded, not a correctness claim.

The Nickel contract and `validate_spike_decision` reject:

- a `proves` claim;
- a keep or replace verdict with missing numbers;
- a replace verdict that the recorded numbers contradict.

Negative fixtures cover each of these, plus a marble default, an unbounded
queue, and overlapping identifier ranges.

## Open

- **ChaosControl campaign references.** The proposal premises a link to the
  F-series fault set. The F01–F14 findings
  (`docs/audits/logical-bugs-2026-09-08.md`) are source-level logic bugs in
  shutdown, scheduling, replication, and merge, not physical-storage faults,
  and no ChaosControl campaign exists for them or for this backend. No campaign
  reference can be recorded truthfully; the task stays open.

## Non-claims

- The spike changes no dataspace, vat, networking, or replication semantics.
- It does not change the default storage path.
- Marble object identifiers are physical handles, not content identity.
- Measurements record one workload on one host and do not prove runtime
  superiority.
- Durability covers only batches marble atomically recovered.

## Gates run

- `cargo test --workspace --lib`: 1725 (`molten`) and 599 (`molten-core`)
  passed. The feature-gated Prolly suite: 26 passed, 1 ignored.
- Clippy with `-D warnings` is clean for the workspace and for `molten` with
  `--features marble-object-store`.
- Octet is clean for the workspace and for `molten` with the feature.
- `nix build` of `marble-store-config`, `release-dependency-profile`, and
  `git-source-hash-binding` succeeded. `scripts/git-source-hashes.sh --check`
  passed.
- Cairn sync created `.cairn/specs/marble-object-store`. Cairn strict
  validation reported no spec or substance findings. The tracey guard passed
  with 2944 requirements, 1010 referenced, and 0 dangling.
