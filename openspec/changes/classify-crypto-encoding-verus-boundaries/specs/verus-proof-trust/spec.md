## ADDED Requirements

### Requirement: Explicit Crypto and Encoding Trust Boundary Classification

Aspen MUST classify residual `external_body` markers that depend on cryptographic primitives, encoding libraries, or collision-resistance assumptions as explicit trusted boundaries, while proving all local shape/admission facts that do not require those assumptions.

#### Scenario: Hash and MAC assumptions are explicit
- GIVEN a Verus spec models Blake3, HMAC, commit IDs, chain hashes, or MAC sensitivity
- WHEN the trust-boundary classification is completed
- THEN each residual marker MUST have a local comment explaining the external assumption and the runtime/library surface that backs it
- AND provable length, fixed-shape, admission, or deterministic wrapper facts MUST be verified where they do not require cryptographic security assumptions

#### Scenario: Tuple encoding assumptions are minimized
- GIVEN `tuple_spec.rs` models order preservation, roundtrip behavior, and tuple comparison laws
- WHEN tuple proof boundaries are classified
- THEN pure structural facts MUST be proved where feasible
- AND remaining tuple encoding/order axioms MUST be named as encoding-library assumptions with runtime test coverage or an explicit follow-up task

#### Scenario: Inventory distinguishes axioms from gaps
- GIVEN the repository still contains `external_body` markers after classification
- WHEN the proof-gap inventory is regenerated
- THEN each remaining crypto/encoding marker MUST be listed as an intentional axiom, not an untriaged proof gap
