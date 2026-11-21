## ADDED Requirements

### Requirement: Dive independently corroborates consensus fast-path model parity
r[molten.consensus.fast_path_model.dive_parity] The x86_64 Linux consensus fast-path model parity check MUST run pinned Dive 0.1.0 in addition to the existing TLC and TLA-ts runs, using the same unchanged model and configurations. TLC MUST remain authoritative. The campaign MUST preserve the 180 second checker/config and 720 second total limits.

#### Scenario: Three exhaustive baseline results agree
- GIVEN the existing finite baseline configuration with its temporal property
- WHEN all three pinned checkers run within the agreed limits
- THEN all MUST report success with an empty queue and the same generated and distinct counts
- AND Dive MUST report a completed, exhausted temporal check.

#### Scenario: Three bounded mutant results agree
- GIVEN the existing missing-promise configuration
- WHEN the checkers exercise the mutant
- THEN each MUST report `NoMissingPromise` with a causal trace of more than one state
- AND early-stop counts MUST remain observations, not parity assertions.

#### Scenario: Unqualified observations fail closed
- GIVEN archive or version drift, an unknown schema, a refusal, a stop, a malformed or incomplete report, a timeout, or a parity mismatch
- WHEN the campaign evaluates the checker observations
- THEN it MUST fail with the named cause and MUST NOT claim parity.

#### Scenario: Evidence remains finite-model-only
- GIVEN retained campaign evidence
- WHEN a reader inspects the result
- THEN it MUST identify the Dive pin, options and durations and bind the raw checker outputs and reports with BLAKE3
- AND agreement with TLC MUST NOT be presented as proof of Aspen's Rust implementation.
