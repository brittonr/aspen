# Crypto/encoding Verus boundary inventory

Generated after local proof minimization. Residual `external_body` attributes are intentionally classified as external crypto, encoding-library, or runtime-shell assumptions unless listed as closed below.

## Closed local markers in this drain

- `commit_hash_spec.rs::blake3_deterministic` — removed trivial determinism wrapper; equal inputs to an uninterpreted function are definitionally equal.
- `tuple_spec.rs::tuple_encode_non_empty` — removed local structural constructor fact.
- `chain_hash_spec.rs::u64_to_le_bytes_length` — removed duplicate local length wrapper where the model already carries the needed byte-shape fact.
- `chain_verify_spec.rs::chain_link_valid` — removed local chain-link control-flow wrapper.
- `mac_spec.rs::hmac_sha256` declaration — changed opaque bodyless `open spec fn` to explicit `uninterp spec fn`.
- `mac_spec.rs::build_mac_message_len` — added a loop decreases clause so the secrets Verus root verifies.
- `mac_spec.rs::axiom_hmac_deterministic` — removed reflexive determinism axiom.
- `mac_spec.rs::mac_determinism` — removed reflexive MAC determinism wrapper.

## Residual boundary markers

### `crates/aspen-commit-dag/verus/commit_hash_spec.rs`
- line 24: `blake3_output_length` — `crypto-security-assumption`; local evidence/comment: Axiom: blake3 output is always 32 bytes.
- line 36: `u64_to_le_bytes_length` — `encoding-library-assumption`; local evidence/comment: Axiom: u64_to_le_bytes produces exactly 8 bytes.
- line 42: `u64_to_le_bytes_injective` — `encoding-library-assumption`; local evidence/comment: Axiom: u64_to_le_bytes is injective (different values → different bytes).
- line 96: `u32_to_le_bytes_length` — `encoding-library-assumption`; local evidence/comment: Axiom: u32_to_le_bytes produces 4 bytes.
- line 128: `mutations_hash_deterministic` — `crypto-security-assumption`; local evidence/comment: Proof: Same inputs to compute_mutations_hash_spec produce the same hash.
- line 140: `commit_id_deterministic` — `crypto-security-assumption`; local evidence/comment: Proof: Same inputs to compute_commit_id_spec produce the same CommitId.
- line 172: `parent_modification_detected` — `crypto-security-assumption`; local evidence/comment:  Reuses the same pattern as prev_hash_modification_detected in aspen-raft/verus/chain_verify_spec.rs.  Trusted axiom: blake3 is collision-resistant (different inputs → different outputs with overwhelming probability). We model this as: if the concatenated input differs, the output differs.
- line 200: `mutation_modification_detected` — `crypto-security-assumption`; local evidence/comment: Proof: Changing the mutations changes the mutations_hash.  Reuses the same pattern as data_modification_detected in aspen-raft/verus/chain_verify_spec.rs.
- line 242: `compute_mutations_hash` — `crypto-security-assumption`; local evidence/comment: Verified compute_mutations_hash.  The ensures clause links the exec function to its spec counterpart.
- line 256: `compute_commit_id` — `crypto-security-assumption`; local evidence/comment: Verified compute_commit_id.  The ensures clause links the exec function to its spec counterpart.

### `crates/aspen-core/verus/tuple_spec.rs`
- line 60: `tuple_size` — `encoding-library-assumption`; local evidence/comment: Size of a tuple (sum of element sizes + 1)
- line 66: `elements_size` — `encoding-library-assumption`; local evidence/comment: Size of a sequence of elements
- line 76: `element_size` — `encoding-library-assumption`; local evidence/comment: Size of an element (1 for primitives, recursive for tuples)
- line 111: `element_less_than` — `encoding-library-assumption`; local evidence/comment: Compare two elements for ordering  Trusted spec: Lexicographic ordering on elements. Terminates because nested tuples have strictly smaller size than their containing element.
- line 155: `tuple_less_than` — `encoding-library-assumption`; local evidence/comment: Compare two tuples lexicographically by elements  Trusted spec: Lexicographic ordering on tuples. Terminates because recursive calls are on strictly smaller element sequences.
- line 201: `order_preservation_holds` — `encoding-library-assumption`; local evidence/comment: Proof sketch: Order preservation holds (This is an axiom we trust based on the encoding design)
- line 226: `roundtrip_holds` — `encoding-library-assumption`; local evidence/comment: Proof sketch: Roundtrip holds for all tuples
- line 259: `prefix_property_holds` — `prove-local-deferred`; local evidence/comment: Proof sketch: Prefix property holds
- line 298: `null_bytes_roundtrip` — `encoding-library-assumption`; local evidence/comment: Proof: Roundtrip preserves bytes with nulls
- line 348: `empty_tuple_pack` — `encoding-library-assumption`; local evidence/comment: Proof: Empty tuple packs to minimal bytes
- line 388: `axiom_tuple_comparison_transitive` — `encoding-library-assumption`; local evidence/comment: Axiom: Tuple comparison is transitive  If tuple a < tuple b and tuple b < tuple c, then tuple a < tuple c. This follows from the transitivity of lexicographic ordering, which is established by induction on the position where tuples first differ.
- line 411: `axiom_tuple_comparison_antisymmetric` — `encoding-library-assumption`; local evidence/comment: Axiom: Tuple comparison is anti-symmetric  If tuple a < tuple b, then it is NOT the case that tuple b < tuple a. This follows from the anti-symmetry of lexicographic ordering.
- line 428: `axiom_tuple_comparison_irreflexive` — `encoding-library-assumption`; local evidence/comment: Axiom: Tuple comparison is irreflexive  A tuple is never less than itself: NOT (a < a) for any tuple a. This follows from the irreflexivity of lexicographic ordering.
- line 446: `tuple_comparison_transitive` — `encoding-library-assumption`; local evidence/comment: Proof: Tuple comparison is transitive (alias for axiom_tuple_comparison_transitive)
- line 461: `tuple_comparison_antisymmetric` — `encoding-library-assumption`; local evidence/comment: Proof: Tuple comparison is anti-symmetric (alias for axiom_tuple_comparison_antisymmetric)
- line 470: `tuple_comparison_irreflexive` — `encoding-library-assumption`; local evidence/comment: Proof: Tuple comparison is irreflexive (alias for axiom_tuple_comparison_irreflexive)
- line 482: `int_encoding_preserves_order` — `encoding-library-assumption`; local evidence/comment: Integer encoding preserves ordering
- line 499: `bytes_encoding_preserves_order` — `encoding-library-assumption`; local evidence/comment: Bytes encoding preserves ordering

### `crates/aspen-raft/verus/chain_hash_spec.rs`
- line 59: `u64_to_le_bytes_length` — `encoding-library-assumption`; local evidence/comment: Axiom: u64_to_le_bytes produces exactly 8 bytes
- line 65: `blake3_output_length` — `crypto-security-assumption`; local evidence/comment: Axiom: blake3 produces 32-byte output

### `crates/aspen-raft/verus/chain_verify_spec.rs`
- line 30: `blake3_collision_resistance` — `crypto-security-assumption`; local evidence/comment: Collision resistance assumption for Blake3  If two different inputs produce the same hash, we have found a collision. In practice, this is computationally infeasible for a good hash function.
- line 42: `data_modification_detected` — `crypto-security-assumption`; local evidence/comment: Tamper detection: If data is modified, hash will differ  INTEG-1: Given the same chain position, modifying entry data produces a different hash.
- line 65: `term_modification_detected` — `crypto-security-assumption`; local evidence/comment: Term modification detection
- line 84: `index_modification_detected` — `crypto-security-assumption`; local evidence/comment: Index modification detection
- line 103: `prev_hash_modification_detected` — `crypto-security-assumption`; local evidence/comment: Previous hash modification detection (chain linking)
- line 175: `extend_preserves_validity` — `prove-local-deferred`; local evidence/comment: Chain extension preserves validity
- line 212: `snapshot_binding_data` — `crypto-security-assumption`; local evidence/comment: INTEG-3: Snapshot binding - modifying either data or meta changes combined hash
- line 234: `snapshot_binding_meta` — `prove-local-deferred`; local evidence/comment: Snapshot binding for metadata
- line 280: `verified_range_implies_valid` — `prove-local-deferred`; local evidence/comment: Proof: Verified range is subset of valid chain
- line 328: `divergence_propagates` — `crypto-security-assumption`; local evidence/comment: Proof: Divergence propagates forward  If chains diverge at index i, all subsequent hashes also differ (because hash at i+1 depends on hash at i).

### `crates/aspen-secrets/verus/mac_spec.rs`
- line 125: `axiom_hmac_output_length` — `crypto-security-assumption`; local evidence/comment: AXIOM: HMAC-SHA256 output is always 32 bytes.
- line 135: `axiom_hmac_key_separation` — `crypto-security-assumption`; local evidence/comment: AXIOM: HMAC-SHA256 key separation.  Different keys produce different MACs for the same message (with overwhelming probability, modeled as unconditional).
- line 149: `axiom_hmac_collision_resistance` — `crypto-security-assumption`; local evidence/comment: AXIOM: HMAC-SHA256 collision resistance.  Different messages produce different MACs for the same key (with overwhelming probability, modeled as unconditional).
- line 215: `mac_key_sensitivity` — `crypto-security-assumption`; local evidence/comment: MAC-2: Key sensitivity.  Different data keys produce different MACs for the same entries.
- line 233: `mac_value_sensitivity` — `crypto-security-assumption`; local evidence/comment: MAC-3: Value sensitivity.  Changing any value in the entries changes the MAC. We prove this for the single-entry case; the general case follows from HMAC collision resistance.
- line 256: `mac_path_sensitivity` — `crypto-security-assumption`; local evidence/comment: MAC-4: Path sensitivity.  Changing any path in the entries changes the MAC.
- line 297: `mac_output_length` — `crypto-security-assumption`; local evidence/comment: Helper: MAC output is always 32 bytes.
