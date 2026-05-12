## Context

The repo currently has no active OpenSpec changes and a clean `main...origin/main` state. The latest proof-gap drain reduced remaining Verus trusted bodies to three files:

| File | Count | Boundary type |
| --- | ---: | --- |
| `crates/aspen-core/verus/tuple_spec.rs` | 18 | Tuple ordering/encoding/roundtrip plus recursive structural helpers |
| `crates/aspen-raft/verus/chain_verify_spec.rs` | 9 | Blake3 collision-resistance and chain/snapshot tamper detection |
| `crates/aspen-secrets/verus/mac_spec.rs` | 7 | HMAC-SHA256 output/key/collision assumptions and MAC sensitivity wrappers |

Prior direct-removal sweeps have already closed the small scalar/structural markers. These remaining markers require classification before implementation because some are intentionally outside Verus' local proof domain.

## Goals / Non-Goals

**Goals:**

- Make the remaining proof inventory actionable and auditable.
- Remove or narrow structural trusted bodies that do not require crypto/encoding assumptions.
- Keep real cryptographic and uninterpreted encoding assumptions explicit, documented, and backed by runtime/library evidence.
- Preserve small commit slices with crate-local verification.

**Non-Goals:**

- Prove Blake3 collision resistance or HMAC security in Verus.
- Prove FoundationDB-style tuple encoding correctness from the production encoder in one broad change.
- Change runtime behavior unless a spec/body mismatch is discovered and separately verified.
- Treat this OpenSpec artifact package as completing the implementation work.

## Decisions

### 1. Classify before editing residual markers

**Choice:** Start with this OpenSpec instead of deleting more attributes directly.

**Rationale:** The remaining markers are clustered around `pack`/`unpack`, Blake3, and HMAC. These are legitimate trusted boundaries unless the model is refined into smaller structural lemmas.

**Alternative:** Continue blind direct-removal attempts. Rejected because previous negative sweeps already showed these are not uniformly trivial and could produce unsound proof pressure.

**Implementation:** Task 1 regenerates the inventory and records each marker as structural, wrapper-over-axiom, or irreducible trusted boundary.

### 2. Tuple work is the first implementation slice

**Choice:** Tackle `tuple_spec.rs` before chain/MAC.

**Rationale:** Tuple contains the largest count and includes likely structural recursive helpers (`tuple_size`, `elements_size`, `element_size`, possibly lexicographic helper facts) mixed with true encoding axioms. It offers the best chance to reduce marker count without pretending to prove crypto.

**Alternative:** Start with MAC/HMAC. Rejected because most MAC markers are already explicit cryptographic assumptions or wrappers around those assumptions.

**Implementation:** Prove structural size/order helpers where feasible; keep `pack`/`unpack` properties trusted unless a smaller uninterpreted boundary is introduced.

### 3. Chain verification narrows structural facts but preserves Blake3 assumptions

**Choice:** Only prove chain-map/linking facts that do not require hash injectivity.

**Rationale:** Tamper-detection proofs depend on collision resistance/injectivity of `blake3_spec`, which is deliberately uninterpreted. Verus can check shape and map/chain preservation around those assumptions, not the cryptographic property itself.

**Implementation:** Convert wrapper proofs to explicit calls to a named Blake3 assumption where possible; add comments for any residual trusted body stating exactly which hash property is assumed.

### 4. MAC work should remove duplicate wrapper trust

**Choice:** Keep named HMAC axioms trusted, but make sensitivity/output wrapper proofs call those axioms directly where Verus can discharge sequence-message distinctions.

**Rationale:** `mac_spec.rs` already models HMAC as uninterpreted; the durable improvement is to reduce the number of independently trusted wrappers and clarify that remaining assumptions are HMAC properties.

**Implementation:** Preserve `axiom_hmac_*` as boundary candidates, prove `mac_output_length` via `axiom_hmac_output_length`, and attempt single-entry path/value sensitivity via message construction plus `axiom_hmac_collision_resistance`.

## Risks / Trade-offs

**Unsound axiom laundering** → Mitigate by requiring each residual marker to name the external assumption and by rejecting proofs that assert cryptographic injectivity without a named axiom.

**Over-broad tuple proof work** → Mitigate by splitting structural helper closure from full production encoder correctness.

**Runtime evidence drift** → Mitigate with focused Rust tests whenever comments cite runtime/library behavior or any executable helper is touched.

## Validation Plan

- OpenSpec artifact validation: `openspec validate --all --strict`.
- Tuple slice: `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-core/verus/lib.rs` plus focused core/tuple tests if runtime evidence changes.
- Chain slice: `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-raft/verus/lib.rs` plus focused chain/integrity tests if executable contracts change.
- MAC slice: `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-secrets/verus/lib.rs` plus focused MAC/SOPS tests if runtime contracts change.
- Final archive: regenerated inventory shows only documented intentional trusted boundaries or zero residual markers.
