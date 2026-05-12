## ADDED Requirements

### Requirement: Remaining Verus Trusted Boundary Inventory

Aspen MUST maintain a current inventory for the remaining Verus `external_body` markers once direct scalar and structural proof gaps have been drained.

#### Scenario: Residual markers are counted by boundary class

- GIVEN the repository has no active direct proof-gap OpenSpecs
- WHEN the remaining `external_body` inventory is regenerated
- THEN the inventory MUST identify the remaining files and counts
- AND each file MUST be assigned to a boundary class: tuple encoding/order, chain hash verification, or MAC/HMAC cryptographic assumption

#### Scenario: Inventory is not treated as product proof

- GIVEN a marker is classified as an intentional trusted boundary
- WHEN the classification is reported
- THEN the report MUST NOT claim that Verus proves the cryptographic or external encoding property itself
- AND it MUST name the runtime/library/test evidence that backs the trusted assumption.

### Requirement: Trusted Boundary Reduction Before Axiom Retention

Aspen MUST attempt to reduce each residual trusted boundary to the smallest sound assumption before leaving an `external_body` marker in place.

#### Scenario: Tuple structural helpers are separated from encoding axioms

- GIVEN `tuple_spec.rs` contains recursive size/order helpers and tuple encoding properties
- WHEN tuple boundary work is implemented
- THEN recursive structural helpers SHOULD be proved directly where Verus accepts a terminating model
- AND order-preservation, pack/unpack roundtrip, prefix, null escaping, and empty-pack claims MAY remain trusted only when they depend on the uninterpreted `pack`/`unpack` encoding boundary.

#### Scenario: Chain verification separates byte-shape lemmas from Blake3 assumptions

- GIVEN `chain_verify_spec.rs` contains Blake3 collision-resistance and tamper-detection proofs
- WHEN chain verification boundary work is implemented
- THEN byte-shape and chain-map structural lemmas SHOULD be proved when possible
- AND collision-resistance or hash-injectivity claims MUST remain explicit trusted assumptions tied to Blake3/runtime hash evidence.

#### Scenario: MAC wrapper proofs delegate only to named HMAC axioms

- GIVEN `mac_spec.rs` contains HMAC output, key-separation, collision-resistance, and sensitivity proofs
- WHEN MAC boundary work is implemented
- THEN wrapper proofs SHOULD call the smallest named HMAC axiom rather than remain independently trusted
- AND pure message-construction, empty-input, sorting, and length facts SHOULD be verified directly where possible.

### Requirement: Residual Boundary Verification Evidence

Aspen MUST attach concrete verification evidence to every completed residual-boundary slice.

#### Scenario: Core tuple slice evidence

- GIVEN tuple spec changes are made
- WHEN the slice is completed
- THEN `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-core/verus/lib.rs` MUST pass
- AND focused tuple/runtime tests MUST pass if runtime tuple behavior or comments claiming test evidence are changed.

#### Scenario: Raft chain verification slice evidence

- GIVEN chain verification spec changes are made
- WHEN the slice is completed
- THEN `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-raft/verus/lib.rs` MUST pass
- AND focused chain/hash/integrity tests MUST pass if executable helpers or runtime-facing contracts are changed.

#### Scenario: Secrets MAC slice evidence

- GIVEN MAC spec changes are made
- WHEN the slice is completed
- THEN `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-secrets/verus/lib.rs` MUST pass
- AND focused MAC/SOPS tests MUST pass if runtime MAC behavior or evidence anchors are changed.

## MODIFIED Requirements

### Requirement: Explicit Crypto and Encoding Trust Boundary Classification

Aspen MUST classify residual `external_body` markers that depend on cryptographic primitives, encoding libraries, or collision-resistance assumptions as explicit trusted boundaries, while proving all local shape/admission/structural facts that do not require those assumptions.

#### Scenario: Hash and MAC assumptions are explicit

- GIVEN a Verus spec models Blake3, HMAC, chain hashes, or MAC sensitivity
- WHEN trust-boundary classification is completed
- THEN each residual marker MUST have a local comment explaining the external assumption and the runtime/library surface that backs it
- AND provable length, fixed-shape, admission, or deterministic wrapper facts MUST be verified where they do not require cryptographic security assumptions.

#### Scenario: Tuple encoding assumptions are minimized

- GIVEN `tuple_spec.rs` models order preservation, roundtrip behavior, prefix behavior, null escaping, and tuple comparison laws
- WHEN tuple proof boundaries are classified
- THEN pure structural facts MUST be proved where feasible
- AND remaining tuple encoding/order axioms MUST be named as encoding assumptions with runtime test coverage or an explicit follow-up task.

#### Scenario: Inventory distinguishes axioms from gaps

- GIVEN the repository still contains `external_body` markers after classification
- WHEN the proof-gap inventory is regenerated
- THEN each remaining crypto/encoding marker MUST be listed as an intentional axiom, not an untriaged proof gap.
