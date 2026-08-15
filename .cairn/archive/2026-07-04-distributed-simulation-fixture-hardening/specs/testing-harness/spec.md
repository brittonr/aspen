## ADDED Requirements

### Requirement: Distributed simulation direct fault fixtures are complete
r[molten.testing.distributed_simulation.direct_fault_fixtures] Molten SHOULD test every supported deterministic simulation fault class with a named fixture that asserts the expected decision, committed operation ids, denied operation ids, event kind, diagnostic, final-state ref, and run receipt ref stability.

#### Scenario: Benign faults preserve deterministic commits
- GIVEN admitted workflow commands under declared delay, drop, reorder, rejoin, crash, restart, or duplicate-delivery fault events
- WHEN the simulator runs twice with the same topology, scheduler profile, seed, commands, and fault plan
- THEN accepted operations emit stable event refs, final-state refs, diagnostics, and run receipt refs
- AND benign fault diagnostics name the active fault without granting authority, transport, policy, provenance, resource, source-gate, retention, deployment, or production-readiness trust.

#### Scenario: Denial faults stop before side effects
- GIVEN workflow commands exposed to stale-evidence, corrupted-receipt, resource-pressure, unauthorized-transport, undeclared-ambient-state, or partitioned-quorum fault events
- WHEN the simulator evaluates the commands
- THEN each affected command denies before side effects
- AND the run records denied operation ids, no semantic commit for denied commands, and fault-specific diagnostics in the canonical run receipt.

#### Scenario: Fixture drift changes canonical evidence
- GIVEN a passing direct fault fixture and a mutated peer id, operation id, fault kind, schedule field, payload ref, or required evidence ref
- WHEN the simulator canonicalizes both inputs
- THEN the mutated fixture changes the relevant topology, fault-plan, event, final-state, or run receipt ref
- AND any missing required evidence ref fails closed rather than reusing pass evidence.

### Requirement: Distributed simulation fixture traceability is explicit
r[molten.testing.distributed_simulation.fixture_traceability] Molten SHOULD bind the direct distributed simulation fixture set to traceability markers that identify positive and negative coverage commands, artifact refs or receipt refs, and the requirement ids covered by each fixture family.

#### Scenario: Fixture coverage names positive and negative evidence
- GIVEN distributed simulation requirements that claim direct fixture coverage
- WHEN traceability is scanned for release or review
- THEN positive fixtures and negative fixtures are both visible with command evidence
- AND missing, stale, unsupported, or diagnostic-only evidence cannot satisfy pass coverage.

### Requirement: Distributed CI profile wiring evidence follows configured profiles
r[molten.testing.distributed_ci.profile_wiring_evidence] Molten SHOULD test distributed CI metadata and gate fixtures against the configured distributed CI profile matrix. Profile ids, command surfaces, expected artifact kinds, cost classes, release-review statuses, retry policy, unavailable handling, and variance declarations MUST come from the configured matrix or an explicit reviewed fixture derived from it.

#### Scenario: Profile metadata follows the configured matrix
- GIVEN the configured distributed CI profile matrix
- WHEN metadata fixtures are built for fast, protocol, CLI, VM smoke, VM fault, and soak profiles
- THEN each metadata fixture binds the configured profile id, command surface, expected artifact kind, source or tree ref, test binary or package ref, topology ref, seed ref, fault-plan ref, receipt refs, variance refs, and diagnostic log refs
- AND profile metadata remains reproducible without reading ambient runtime state.

#### Scenario: Miswired profile evidence is denied
- GIVEN metadata for a missing profile id, mismatched command surface, missing receipt ref, missing variance declaration, unavailable required profile, or retry-only pass
- WHEN the distributed CI gate evaluates the run
- THEN the gate denies before accepting release pass evidence
- AND diagnostics identify the profile wiring error that must be fixed or explicitly exempted.
