# verus-proof-trust Specification

## Purpose
TBD - created by archiving change close-raft-batcher-apply-verus-proofs. Update Purpose after archive.
## Requirements
### Requirement: Raft Non-Crypto Verus Proof Gap Closure

Aspen MUST close remaining Raft `external_body` markers that represent operational arithmetic, byte accounting, or batch contiguity rather than cryptographic chain assumptions.

#### Scenario: Apply request arithmetic is verified
- GIVEN `apply_request_spec.rs` has trusted version/last-applied arithmetic facts
- WHEN the apply proof slice is implemented
- THEN the Raft Verus root MUST pass
- AND the markers for version increment and batch last-applied update MUST be removed or narrowed with a documented model limitation

#### Scenario: Write-batcher accounting is verified
- GIVEN `batcher_add_spec.rs` and `batcher_flush_spec.rs` have trusted byte-count and contiguity facts
- WHEN the batcher proof slice is implemented
- THEN the Raft Verus root MUST pass
- AND focused write-batcher tests MUST pass if executable helper semantics are touched

#### Scenario: Crypto chain assumptions remain out of scope
- GIVEN Raft chain hash/verify specs contain cryptographic assumptions
- WHEN this change is completed
- THEN those crypto assumptions MUST be left to the crypto/encoding boundary OpenSpec, not mixed with operational batcher/apply proof closure

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

### Requirement: Structural Verus Proof Gap Closure

Aspen MUST eliminate or narrow the remaining structural `#[verifier(external_body)]` trust markers that represent collection, FIFO, index, and invariant-preservation facts rather than cryptographic assumptions.

#### Scenario: Queue FIFO and invariant closure
- GIVEN `queue_ack_spec.rs` contains trusted FIFO or redrive invariant proof bodies
- WHEN the queue proof slice is implemented
- THEN `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-coordination/verus/lib.rs` MUST pass
- AND `queue_ack_spec.rs` MUST have no structural FIFO/invariant `external_body` marker unless a residual marker is explicitly classified with a blocker comment and a narrower OpenSpec follow-up

#### Scenario: Registry worker strategies and fencing closure
- GIVEN coordination registry, worker, strategies, and fencing specs contain trusted structural or arithmetic helper proofs
- WHEN each proof slice is implemented
- THEN the coordination Verus root MUST pass
- AND focused Rust tests for the touched domain MUST pass when runtime helpers or semantics are touched

#### Scenario: Core directory and index closure
- GIVEN core directory/index specs contain trusted Map/Set/Seq invariant facts
- WHEN the core proof slice is implemented
- THEN `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-core/verus/lib.rs` MUST pass
- AND the affected trusted markers MUST be removed or narrowed to a documented non-structural boundary

#### Scenario: Commit diff validity closure
- GIVEN `diff_spec.rs` contains trusted order and validity facts
- WHEN diff proof helper lemmas are added
- THEN the commit-dag Verus root MUST pass
- AND diff validity markers MUST be removed or documented as a precise model limitation
