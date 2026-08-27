# Molten World Commit Specification Delta

## Purpose

Measure structural sharing, mutation amplification, replication reuse, and retention planning before selecting a new branchable-state component.

## ADDED Requirements

### Requirement: Benchmark profiles are typed and content-bound

r[molten.world_bench.profile] Molten MUST define typed Nickel benchmark profiles that bind dataset, preparation, operation sequence, warm or cold state, profile class, adapters, bounds, repetitions, hardware cohort, and named acceptance thresholds. Rust MUST revalidate every projection before execution.

#### Scenario: Complete profile is repeated

- GIVEN the same validated profile, source revision, dataset, adapters, and preparation state
- WHEN two benchmark plans are identified
- THEN they MUST have the same canonical plan identity

#### Scenario: Threshold is unexplained

- GIVEN a profile contains a nontrivial numeric threshold without a named typed field
- WHEN profile validation runs
- THEN Molten MUST reject the profile

### Requirement: Structural sharing metrics use exact facts

r[molten.world_bench.metrics] Required results MUST record logical bytes, physical bytes written, new objects, reused objects, copied pages, mapped pages, traversed references, compared keys, emitted conflicts, transferred bytes, retained objects, and planned deletions where applicable. Missing required metrics MUST invalidate the result.

#### Scenario: Branch starts from unchanged roots

- GIVEN a branch is created without mutation
- WHEN branch cost is measured
- THEN the result MUST distinguish root-reference publication from copied bytes and new object materialization

#### Scenario: Physical bytes are reported as logical bytes

- GIVEN compression or deduplication makes physical and logical byte counts differ
- WHEN receipt validation runs
- THEN Molten MUST reject a result that collapses the two metrics

### Requirement: Preparation state is explicit

r[molten.world_bench.datasets] Dataset construction, prepopulation, cache warming, compaction, and prior object availability MUST be identified before measured operations. Unknown preparation state MUST block accepted comparison.

#### Scenario: Warm cache is undeclared

- GIVEN objects are available from an earlier run but the profile declares cold state
- WHEN benchmark admission runs
- THEN the result MUST be rejected as preparation drift

### Requirement: Snapshot profile results remain distinct

r[molten.world_bench.snapshot_profiles] Logical typed-state and opaque machine-snapshot benchmark cohorts MUST use distinct result classes. Molten MUST NOT claim semantic or performance equivalence across those classes.

#### Scenario: Opaque snapshot reports fewer written bytes

- GIVEN one opaque fixture writes fewer bytes than one logical fixture
- WHEN results are summarized
- THEN the summary MUST NOT claim the opaque profile is universally superior or semantically equivalent

### Requirement: Retention benchmarks do not grant deletion authority

r[molten.world_bench.retention] Retention measurements MAY cover reachability, pin evaluation, protected-object reuse, candidate classification, and deletion-plan size. The benchmark verdict MUST NOT authorize deletion.

#### Scenario: Reachable object enters a deletion plan

- GIVEN an object remains reachable from an admitted world, branch, witness, capsule, quarantine, or policy root
- WHEN retention correctness validation runs
- THEN the result MUST fail regardless of benchmark speed

### Requirement: Extraction decisions require accepted evidence

r[molten.world_bench.extraction_decision] A pure classifier MUST return retain-current, optimize-in-place, or evaluate-shared-component from accepted benchmark receipts and typed policy. It MUST require repeated product-neutral limits and at least two credible consumers before recommending shared-component evaluation.

#### Scenario: One Molten fixture misses a duration target

- GIVEN only one product-specific timing result misses policy
- WHEN extraction classification runs
- THEN it MUST NOT recommend automatic creation of a shared repository

### Requirement: Benchmark receipts preserve finite-run limits

r[molten.world_bench.receipt] Receipts MUST bind source revision, profile, dataset, preparation, adapters, hardware cohort, limits, exact metrics, duration observations, results, and unsupported rows. They MUST NOT claim asymptotic proof, universal performance, storage correctness, or release eligibility.

#### Scenario: Receipt claims big-O proof

- GIVEN a finite benchmark receipt claims it proved asymptotic complexity
- WHEN receipt validation runs
- THEN Molten MUST reject the overclaim

### Requirement: Benchmark verification covers misleading inputs

r[molten.world_bench.verification] Molten MUST test stable results, structural reuse, mutation accounting, profile separation, preparation drift, missing metrics, stale revisions, unsafe retention, and extraction overclaims.

#### Scenario: Focused benchmark rail runs

- GIVEN positive and negative fixtures use one reviewed cohort
- WHEN benchmark verification runs
- THEN it MUST report exact metric coverage and all bounded non-claims
