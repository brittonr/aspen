# verus-proof-trust Specification

## Purpose

This specification separates machine-checked Verus proof obligations from intentional trusted boundaries. Aspen MUST drain direct structural proof gaps where feasible, while retaining only narrowly named assumptions for cryptographic collision resistance, opaque byte encodings, and production tuple encoder semantics that Verus does not model directly.

## Current residual trusted-boundary inventory

As of 2026-05-12, the source-derived residual `external_body` inventory is 11 markers:

| File | Count | Boundary class | Residual assumption |
|------|------:|----------------|---------------------|
| `crates/aspen-commit-dag/verus/commit_hash_spec.rs` | 1 | BLAKE3 cryptographic boundary | `blake3_collision_resistance` for different encoded commit/hash inputs. |
| `crates/aspen-core/verus/tuple_spec.rs` | 6 | Tuple encoding/order boundary | Production FoundationDB-style tuple encoder order preservation, roundtrip, prefix, NUL escaping, integer order encoding, and byte-array order encoding. |
| `crates/aspen-raft/verus/chain_verify_spec.rs` | 2 | BLAKE3 + u64 encoding boundary | `blake3_collision_resistance` and `u64_to_le_bytes_injective`; all local chain/tamper wrappers around these assumptions are verified. |
| `crates/aspen-secrets/verus/mac_spec.rs` | 2 | HMAC cryptographic boundary | HMAC-SHA256 key separation and collision resistance; local MAC sensitivity wrappers are verified. |

This inventory is not a claim that Verus proves BLAKE3, HMAC-SHA256, or the production tuple encoder. It records the remaining trusted assumptions and the files whose wrapper proofs reduce to those assumptions.

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
- AND remaining tuple encoding/order axioms MUST be named as encoding assumptions with runtime test coverage anchored by `cargo test -p aspen-layer`, including `test_trusted_tuple_boundary_runtime_evidence`, `prop_roundtrip`, `prop_string_ordering`, `prop_bytes_ordering`, `prop_int_ordering`, `prop_prefix_stability`, and `prop_range_captures_prefix` (or an explicit follow-up task if any anchor is missing).

#### Scenario: Inventory distinguishes axioms from gaps

- GIVEN the repository still contains `external_body` markers after classification
- WHEN the proof-gap inventory is regenerated
- THEN each remaining crypto/encoding marker MUST be listed as an intentional axiom, not an untriaged proof gap.

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
