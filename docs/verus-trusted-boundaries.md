# Verus trusted boundaries

Status: 2026-05-12 operator/reviewer summary.

Aspen's direct structural Verus proof gaps have been drained or narrowed. The remaining `#[verifier(external_body)]` markers are intentional trusted boundaries for properties that this Verus model does not prove directly: cryptographic collision/security assumptions, fixed byte encoding injectivity, and production tuple encoder semantics.

This is **not** a claim that Verus proves BLAKE3, HMAC-SHA256, or the FoundationDB-style tuple encoder. It is a bounded inventory of the assumptions left outside the proof kernel, plus the runtime/library evidence that backs each assumption.

## Residual inventory

| File | Markers | Boundary class | Assumption retained | Evidence |
|------|--------:|----------------|---------------------|----------|
| `crates/aspen-commit-dag/verus/commit_hash_spec.rs` | 1 | BLAKE3 cryptographic boundary | Different encoded commit/hash inputs do not collide under BLAKE3. | Local wrappers reduce input construction before calling `blake3_collision_resistance`; runtime evidence is anchored by `trusted_blake3_commit_boundary_runtime_evidence`. |
| `crates/aspen-core/verus/tuple_spec.rs` | 6 | Tuple encoding/order boundary | Production tuple encoding preserves order, roundtrips, prefix ranges, NUL escaping, integer order encoding, and byte-array order encoding. | `cargo test -p aspen-layer`, including `test_trusted_tuple_boundary_runtime_evidence`, `prop_roundtrip`, `prop_tuple_ordering`, `prop_string_ordering`, `prop_bytes_ordering`, `prop_int_ordering`, `prop_int_boundaries`, `prop_special_strings`, `prop_prefix_stability`, and `prop_range_captures_prefix`. |
| `crates/aspen-raft/verus/chain_verify_spec.rs` | 2 | BLAKE3 + `u64` encoding boundary | Different chain/hash inputs do not collide under BLAKE3, and little-endian `u64` encoding is injective. | Local chain/tamper wrappers are verified around these assumptions; runtime evidence is anchored by `test_trusted_blake3_chain_boundary_runtime_evidence`. |
| `crates/aspen-secrets/verus/mac_spec.rs` | 2 | HMAC cryptographic boundary | HMAC-SHA256 has key separation and collision resistance for the modeled MAC messages. | Local MAC message construction and sensitivity wrappers delegate to named HMAC axioms; runtime evidence is anchored by `test_trusted_hmac_boundary_runtime_evidence`. |

Total: 11 source-level residual `external_body` attributes across 4 Verus files.

## Reviewer rule

Treat each marker as an external assumption, not as product proof. A future change may remove or refine a marker only if it either:

1. proves the local structural fact in Verus without broadening the assumption surface, or
2. replaces it with a narrower named axiom and preserves equivalent runtime/library evidence.

## Regeneration check

Use this command to regenerate the source-level inventory without matching ordinary comments:

```bash
rg -n '#\[verifier(?:::|\()external_body' crates/*/verus/*.rs
```

Expected grouped count on this snapshot:

```text
1  crates/aspen-commit-dag/verus/commit_hash_spec.rs
6  crates/aspen-core/verus/tuple_spec.rs
2  crates/aspen-raft/verus/chain_verify_spec.rs
2  crates/aspen-secrets/verus/mac_spec.rs
```

Primary spec: `openspec/specs/verus-proof-trust/spec.md`.
