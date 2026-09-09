# Outside-in fault track design

## Boundary

The track owns process-level fault execution against the real stack. It does not own deterministic simulation, consensus campaigns, or any product runtime component. The harness is a testing-infrastructure surface: it starts real processes, injects bounded faults, collects observations, and stops cleanly.

## Proposed decision

The harness drives each scenario in three parts.

**Setup.** Start the declared node processes on the real Iroh transport with real local Redb stores, using ordinary public entry points and a distinct named deployment profile. No test-only product code paths are added.

**Fault phase.** Apply one declared profile: kill at a stated boundary, pause and resume, partition-and-heal on the real transport, or a lost response at a stated client boundary. Fault boundaries are declared per scenario; the harness does not explore unbounded fault space.

**Observation phase.** After stabilization, read externally observable histories through public interfaces: query results, delivery outcomes, replication status, promotion outcomes. Where the protocol-oracle work publishes projections, collect them alongside, but agreement is judged on externally observed histories. No physical-timing comparison.

Scenario results are typed: agreement, permitted divergence with recorded reason, unsupported behavior, or incomplete observation. Uncertainty survives: a lost response records an unresolved outcome, never an invented success.

## Resource bounds and hygiene

Every scenario declares bounded process counts, wall-clock and virtual budgets, and output-retention limits before it starts. The harness owns teardown of the processes and scratch paths it created and preserves logs from failed scenarios.

## Compatibility and coordination

The consensus path is out of scope; ChaosControl owns it. The harness reuses existing transport, storage, and identity configuration surfaces rather than adding parallel configuration formats. `differentiate-live-simulation-conformance` owns adapter-level differential comparison; this track adds whole-process faults above it, with no shared runner assumed until both exist.

## Validation and ownership

Testing-infrastructure maintainers own the harness, fixtures, and docs. Baseline the no-fault scenario before any fault profile. Move each executed scenario into normal repository tests or a declared harness tier with named budgets. Retain the no-production, no-simulation-replacement, and no-performance non-claims. Run focused Octet and Clippy checks on any repository code, workspace tests, relevant Nix checks, and required Cairn gates. This plan grants no implementation permission.
