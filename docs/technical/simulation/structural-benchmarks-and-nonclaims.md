# Structural benchmarks and non-claims

Molten's world-benchmark rail measures bounded structural facts without becoming the owner of the operations being measured. This article explains preparation, metric completeness, receipt acceptance, and the limits of synthetic sharing and retention observations. It assumes the [world benchmark guide](../../world-benchmark-sharing-and-retention.md); the [Technical companion](../README.md) connects these measurements to the broader world-state architecture.

## Measurement is not operation authority

The functional core validates plans, datasets, measurements, comparisons, thresholds, and decisions. Application-owned shell ports prepare datasets, observe operations and resources, bind snapshots, and publish records. Existing world, content, replication, and retention adapters retain their operation authority. A count of planned deletions is not permission to delete; an accepted benchmark is not a replacement for storage correctness evidence.

This distinction is visible in [instrumentation.rs](../../../src/world_benchmark/instrumentation.rs). `instrument_world_benchmark_facts` projects supplied exact counters into metric observations. It rejects empty adapter identity, collapsed logical/physical measurement sources, and any protected deletion candidate in a retention-plan observation. It does not perform the retention operation or discover protected objects itself. Correctly identifying reachable, pinned, witnessed, quarantined, and policy-retained objects remains an obligation of the supplying operation owner.

## Preparation is part of the cohort

The reviewed profiles bind source revision, dataset, preparation, logical or opaque class, operation sequence, repetitions, adapters, hardware cohort, finite bounds, and named thresholds. Rust [projection decoding](../../../src/world_benchmark/projection.rs) denies unknown fields, decodes the exported JSON, and invokes profile and dataset validation before returning the typed inputs. The benchmark is therefore not defined by an operation name and dataset size alone.

The [result validator](../../../crates/molten-core/src/world_benchmark/validation/result.rs) checks the preparation observation against dataset and source identities, rejects unknown preparation, and detects preparation drift. A cold plan with prior objects available is hidden prepopulation. Declared warm preparation is legitimate precisely because prior availability is explicit rather than smuggled into an apparently cold experiment.

This is particularly important when evaluating sharing. Reusing objects can be the intended behavior, but reuse in a cold experiment and reuse of explicitly preexisting warm objects answer different questions. Comparing them without the preparation binding can reward an adapter for work performed before the observed interval.

## Complete metrics, independent interpretations

Every result contains all metric classes, including zeros: logical bytes, physical bytes written, new and reused objects, copied and mapped pages, traversed references, compared keys, emitted conflicts, transferred bytes, retained objects, and planned deletions. The helper `complete_world_benchmark_metrics` supplies zeros for absent values; validation separately checks completeness, duplicates, and per-class bounds. Zero is a measurement value in this record, not an omitted column that reviewers can interpret opportunistically.

Logical and physical bytes remain distinct even if numerically equal. Duration and peak memory are optional secondary observations, not substitutes for structural metrics. Result validation checks adapter membership, operation membership, repetition range, resource bounds, and the independent-physical-measurement flag.

The [deterministic fixture](../../../src/world_benchmark/fixture.rs) makes the evidence limit unusually concrete. Its resource port returns no duration or peak-memory measurements. Its root-branch fixture assigns 64 logical bytes, 64 physical bytes, one new object, and reuse of the dataset's object count. These are synthetic fixture facts, not measurements of a production store's actual writes. Equal counts and an independence flag do not turn synthetic input into live telemetry; the governing guide explicitly requires a live adapter to supply its own physical-byte, duration, and peak-memory observations.

## Worked threshold example

Consider an illustrative complete result matrix with two repetitions of replication. Both repetitions are structurally valid; observed transferred-byte counts are 8,000 and 12,000, and the named threshold permits at most 10,000. The [receipt finalizer](../../../crates/molten-core/src/world_benchmark/validation/receipt.rs) evaluates the maximum across matching rows, so the threshold fails at 12,000.

If there are no unsupported rows, the receipt can nevertheless be accepted. Acceptance means complete, structurally valid evidence; it does not mean every policy threshold passed. This distinction preserves unfavorable results instead of discarding them as invalid measurements. Conversely, supplying the same repetition twice while omitting the other is not a valid complete matrix, even if the total number of rows happens to match.

Now change one run from cold to declared warm. Receipt comparison checks preparation along with class, profile, dataset, source revision, hardware cohort, and adapters. The changed preparation makes the pair unsuitable for the same comparison cohort; an attractive transferred-byte reduction does not override that mismatch.

## Opaque snapshots and extraction limits

Opaque results require a snapshot binding, while logical results reject one. Validation checks the exact ChaosControl revision and completeness profile, together with descriptor-reference shape. The governing guide pins revision `b8c440ea3b19df796542e58e8ee36200e1c3db85` and profile `exact-x86-kvm-v1`. Descriptor validity concerns exact metadata, not demonstrated clone realization, replay, portability, semantic equivalence, or KVM correctness.

The guide's extraction dispositions are similarly bounded: retain the current approach, optimize in place, or evaluate a shared component given the supplied evidence and policy. None creates a repository, approves a dependency, transfers ownership, or authorizes release. Finite structural results cannot establish asymptotic complexity or universal performance.

## Review and verification guidance

Suggested review is to trace preparation and dataset identities into the plan, inspect every operation/repetition row, distinguish accepted evidence from threshold outcomes, and reject cross-cohort comparisons. Exercise protected-deletion, unknown-preparation, missing-metric, and logical/opaque-mixing negatives when changing this rail. Those are proposed checks, not executed benchmark results here.

The evidence discipline resembles [distributed testing](../../distributed-testing.md): diagnostic or synthetic artifacts retain their stated scope. Structural sharing counts do not prove crash safety, and retention-plan counts do not grant cleanup authority; those boundaries also appear in [world-fault conformance](../../world-fault-conformance.md).

## Sources

- [World benchmark sharing and retention](../../world-benchmark-sharing-and-retention.md)
- [Distributed testing evidence](../../distributed-testing.md)
- [World crash and concurrency conformance](../../world-fault-conformance.md)
- [Input projection and revalidation](../../../src/world_benchmark/projection.rs)
- [Instrumentation and protected-deletion checks](../../../src/world_benchmark/instrumentation.rs)
- [Deterministic count-only fixture](../../../src/world_benchmark/fixture.rs)
- [Result and preparation validation](../../../crates/molten-core/src/world_benchmark/validation/result.rs)
- [Receipt finalization, thresholds, and comparison](../../../crates/molten-core/src/world_benchmark/validation/receipt.rs)
- [Technical companion](../README.md)
