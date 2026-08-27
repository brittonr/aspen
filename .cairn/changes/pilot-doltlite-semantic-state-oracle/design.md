## Context

The Molten world-commit roadmap needs evidence from an implementation that does not share the native semantic-state map code. DoltLite is suitable as a bounded oracle because its storage, branch, and compatibility behavior is independently implemented and documented.

The reviewed source is DoltLite commit `10170ed82c1b12414db8d1b29d2fe9ea2a72fd88`. DoltLite extensions are Apache-2.0. SQLite portions are public domain, while build helpers and vendored dependencies retain their stated terms. The pilot must preserve notices and record the exact imported scope.

## Decisions

### Decision: Use DoltLite only through a test-owned oracle port

**Choice:** Molten defines a `SemanticStateOracle` test capability in application terms. The adapter translates canonical semantic keys, values, branch operations, and expected failure classes into DoltLite calls.

The production composition root does not select this adapter. Core crates do not depend on SQLite or DoltLite types.

**Rationale:** The oracle can challenge Molten behavior without moving product meaning or storage authority into a vendor fork.

### Decision: Pin source and disable remotes

**Choice:** The Nix test package pins the exact reviewed commit and preserves its license material. The build disables remote support. Each case receives one disposable capability-rooted directory and a bounded resource profile.

**Rationale:** Network and remote-ref behavior add authority and transport surfaces that this oracle does not need.

### Decision: Compare semantic observations, not backend identities

**Choice:** The adapter emits canonical ordered observations over keys, values, branch-visible state, conflict classes, and operation outcomes. It records DoltLite object IDs only as backend-local evidence.

Cross-backend comparison uses a Molten-owned BLAKE3 identity over normalized observations. It never asserts that a DoltLite root equals a Molten root.

**Rationale:** Different canonical formats can represent the same logical result. Hash equality across unrelated formats would be a false contract.

### Decision: Avoid rowid identity

**Choice:** Oracle schemas use explicit canonical primary keys and deterministic built-in ordering. Tests reject rowid-dependent schemas, custom collations, and unspecified ordering.

**Rationale:** DoltLite intentionally differs from stock SQLite rowid behavior. Ambient row identity would make the oracle unstable and domain-incorrect.

### Decision: Keep a typed compatibility ledger

**Choice:** Nickel owns rows with `id`, source contract, status, evidence, fixture, issue, and explanation fields. Status is one of `compatible`, `adapted`, `intentional`, `unsupported`, or `engine-gap`.

`unsupported` and `engine-gap` rows require a negative fixture and tracked issue. Policy records maximum counts by status. The gate denies count increases or missing evidence.

**Rationale:** A machine-readable ledger makes divergence explicit and prevents silent compatibility erosion.

### Decision: Import contract cases without importing product policy

**Choice:** The pilot covers detached snapshots, stale write upgrades, branch isolation, compare-and-advance races, reader-safe GC, exact format rejection, serialization, and history-independent primary-key state.

Molten keeps durable conflict artifacts, typed merge policy, complete-world roots, authority admission, effect release, and retention decisions. DoltLite behavior that conflicts with these rules is an intentional difference.

**Rationale:** The oracle tests mechanisms. It does not define Molten semantics.

### Decision: Treat differential agreement as bounded evidence

**Choice:** The rail records exact inputs, source and build identities, backend format, observations, and agreement or divergence. A matching result is evidence for that case only.

**Rationale:** Two implementations can share a conceptual defect. Agreement does not prove correctness.

## Verification strategy

Positive cases cover canonical round trips, insertion-order independence, detached reads, isolated branches, successful compare-and-advance, live readers during GC, supported format reopen, and stable normalized observations.

Negative cases cover rowid dependence, custom collation, stale snapshot writes, competing writers, missing pins, tampered storage, wrong format version, malformed serialization, remote enablement, multi-file write assumptions, transient-conflict assumptions, and identity overclaims.

## Rollout

1. Land source, license, and build provenance without enabling production features.
2. Define the Nickel ledger and normalized observation schema.
3. Implement the disposable adapter and baseline contract cases.
4. Add differential use to the Prolly pilot and benchmark rail.
5. Keep the adapter optional until every required negative case passes.

## Claim boundary

The pilot reports behavior of one pinned DoltLite cohort under bounded fixtures. It does not establish SQLite compatibility, Molten correctness, durable conflict safety, complete-world atomicity, remote safety, production readiness, or release eligibility.
