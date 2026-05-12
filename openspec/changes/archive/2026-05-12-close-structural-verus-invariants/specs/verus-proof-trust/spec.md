## ADDED Requirements

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
