## ADDED Requirements

### Requirement: Multinode failure repro bundles are sealed evidence artifacts
r[molten.testing.multinode.failure_repro_bundle] Molten SHOULD export sealed multinode failure repro bundles that bind scenario fixture refs, topology refs, scheduler refs, seed refs, fault-plan refs, command refs, node evidence refs, receipt refs, diagnostics, log refs, redaction policy refs, replay status, and evidence-only caveats.

#### Scenario: Simulation failure bundle replays deterministically
- GIVEN a deterministic distributed simulation failure bundle with stored topology, scheduler, seed, fault plan, commands, and expected invariant
- WHEN the repro verifier replays the stored inputs
- THEN the replay produces the same relevant receipt refs or reports an explicit schema or version mismatch
- AND the bundle remains diagnostic evidence unless a separate gate validates a pass or deny claim.

#### Scenario: VM failure bundle verifies without pretending to replay
- GIVEN a VM failure bundle with platform observations and canonical receipts
- WHEN the repro verifier validates the seal and receipt bindings
- THEN the bundle can verify as non-replayable VM diagnostic evidence
- AND it must not claim deterministic replay if the inputs depend on live platform behavior.

### Requirement: Multinode repro bundles preserve privacy and fail closed
r[molten.testing.multinode.failure_repro_privacy_and_replay] Molten MUST reject tampered, unsealed, stale, private-without-reveal, missing-redaction, or diagnostic-only multinode repro bundles before materializing private content or accepting pass evidence.

#### Scenario: Tampered bundle fails verification
- GIVEN a sealed multinode repro bundle whose topology, fixture, receipt, or redaction manifest has been changed after sealing
- WHEN verify or unpack runs
- THEN verification fails closed before materializing bundle contents.

#### Scenario: Diagnostic bundle cannot satisfy pass gate
- GIVEN a verified failure repro bundle marked diagnostic-only
- WHEN a pass evidence gate evaluates the bundle
- THEN the gate rejects it as pass evidence even if logs appear successful.
