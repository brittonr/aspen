# World benchmark sharing and retention

Molten measures bounded structural facts before it selects a new branchable-state mechanism. The benchmark rail does not own world state, storage, replication, retention, snapshots, or release policy.

## Architecture

`molten-core::world_benchmark` is the functional core. It validates profiles, datasets, exact metrics, comparisons, freshness, thresholds, receipts, and extraction decisions. It performs no I/O and reads no clock.

`molten::world_benchmark` is the imperative shell. Application-owned ports prepare datasets, observe operations, observe resources, bind exact snapshots, and publish receipts. Existing world, content, replication, and retention adapters keep their operation authority.

The deterministic fixture supplies count-only synthetic evidence. It does not claim host latency or memory measurements. A live adapter must supply its own independent physical-byte, duration, and peak-memory observations.

## Profiles and preparation

Nickel owns reviewed profiles under `config/world-benchmark/`. Each profile names every bound and threshold. Nickel rejects unknown preparation, hidden cold-state prepopulation, empty threshold names, and logical or opaque profile mixing.

Rust decodes the exported JSON and revalidates all fields before execution. A profile binds:

- source revision;
- dataset and preparation identities;
- logical or opaque class;
- operation sequence and repetitions;
- adapter and hardware cohorts;
- finite bounds;
- named thresholds.

Cold means that prior benchmark objects are unavailable. Declared warm means that the profile records prior object availability. Unknown preparation cannot produce accepted evidence.

## Exact metrics

Each result contains all metric classes, including zero-valued classes:

- logical bytes;
- physical bytes written;
- new and reused objects;
- copied and mapped pages;
- traversed references;
- compared keys and emitted conflicts;
- transferred bytes;
- retained objects and planned deletions.

Logical and physical measurements remain separate even when their values happen to match. Duration and peak memory are optional secondary observations. They are bound to the hardware cohort when present.

## Instrumented operations

The shell projects exact facts for root-only branch creation, first and repeated mutation, diff, merge planning, capsule export, replication reuse, retention planning, and exact snapshot sharing. It does not replace these operations.

A retention result fails before receipt publication if a reachable, pinned, witnessed, quarantined, or policy-retained object appears as a deletion candidate. Planned deletion count is observation only. It never authorizes deletion.

## Snapshot boundary

Opaque metrics bind the exact ChaosControl snapshot descriptor cohort at revision `7433557b85990f0f07a37ca44b97fef26c2a4c7e` and profile `exact-x86-kvm-v1`.

Descriptor validity binds exact snapshot metadata only. It does not prove clone realization, replay, portability, KVM correctness, semantic equivalence, or release readiness. Logical and opaque receipts cannot enter one comparison cohort.

## Extraction decision

The pure classifier returns one of these bounded dispositions:

- `retain-current` when accepted evidence passes the supplied policy;
- `optimize-in-place` for a bounded owned-adapter or single-consumer limit;
- `evaluate-shared-component` only after repeated product-neutral limits affect the policy-required number of credible consumers.

The decision does not create a repository, approve a dependency, transfer ownership, or authorize a release.

## Receipt interpretation

A receipt binds the plan, source, profile, dataset, preparation, adapters, hardware, limits, exact results, unsupported rows, thresholds, and non-claims. Receipt identity uses domain-separated BLAKE3 over the full bounded record.

An accepted receipt means the measured rows are complete and structurally valid. A failed threshold remains accepted benchmark evidence. Unsupported rows prevent acceptance.

Finite runs do not prove asymptotic complexity, universal performance, storage correctness, future behavior, or release eligibility.
