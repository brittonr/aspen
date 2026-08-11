# Typed direct-evidence manifest for the runtime-spine canonical content-ref batch.
# r[impl molten.project.runtime_spine_content_refs_tracey.direct_repairs]
# r[impl molten.project.runtime_spine_content_refs_tracey.exact_manifest]
# r[impl molten.project.runtime_spine_content_refs_tracey.non_claims]
let Repair = {
  requirement_id | String,
  source_area | String,
  implementation_path | String,
  verification_path | String,
  evidence_scope | String,
}
in
let Rejection = {
  requirement_id | String,
  reason | String,
  counterexample_path | String,
}
in
let Manifest = {
  schema | String,
  observed_date | String,
  base_revision | String,
  accepted_specification | String,
  baseline_before | Number,
  baseline_after | Number,
  runtime_spine_before | Number,
  runtime_spine_after | Number,
  candidate_count | Number,
  repairs | Array Repair,
  rejected_candidates | Array Rejection,
  non_claims | Array String,
}
in
let manifest | Manifest = {
  schema = "molten.runtime-spine-content-refs-tracey-direct-repairs.v1",
  observed_date = "2026-08-10",
  base_revision = "e275a9520795b81c797f5013357d48d5d419477b",
  accepted_specification = "cairn/specs/runtime-spine/spec.md",
  baseline_before = 1943,
  baseline_after = 1933,
  runtime_spine_before = 406,
  runtime_spine_after = 396,
  candidate_count = 12,
  repairs = [
    {
      requirement_id = "molten.runtime_spine.canonical_content_refs.cleanup_tests",
      source_area = "runtime_spine",
      implementation_path = "flake.nix",
      verification_path = "flake.nix",
      evidence_scope = "focused malformed-ref and readback tests plus formatting, Clippy, Octet, Cairn, and full Nix validation",
    },
    {
      requirement_id = "molten.runtime_spine.canonical_content_refs.filename_readback",
      source_area = "runtime_spine",
      implementation_path = "src/ledger/parts/mod/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "ledger and chunk-store filename conversion uses validated shared hex reconstruction and rejects malformed raw digests",
    },
    {
      requirement_id = "molten.runtime_spine.canonical_content_refs.materialized_readback",
      source_area = "runtime_spine",
      implementation_path = "src/ledger/parts/mod/p000/body.rs",
      verification_path = "src/ledger/parts/mod/tests/m000/p000/body.rs",
      evidence_scope = "local reads require materialization and recompute canonical identity before returning content",
    },
    {
      requirement_id = "molten.runtime_spine.canonical_content_refs.migration",
      source_area = "runtime_spine",
      implementation_path = "src/artifacts/parts/mod/p004/body.rs",
      verification_path = "flake.nix",
      evidence_scope = "listed artifact, catalog, coordination, protocol, service, transcript, provenance, redaction, secret, and job validators call the shared parser",
    },
    {
      requirement_id = "molten.runtime_spine.canonical_content_refs.negative_tests",
      source_area = "runtime_spine",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/ledger/parts/mod/tests/m000/p000/body.rs",
      evidence_scope = "malformed, wrong-length, unsupported-algorithm, missing-materialization, and tampered-byte paths fail closed",
    },
    {
      requirement_id = "molten.runtime_spine.canonical_content_refs.node_control",
      source_area = "runtime_spine",
      implementation_path = "src/node/parts/runtime/p002/body.rs",
      verification_path = "src/node/parts/runtime/tests/m000/p000/body.rs",
      evidence_scope = "requests, payloads, ingress envelopes, live receipts, and subreceipts use canonical parsing and recomputed envelope identities",
    },
    {
      requirement_id = "molten.runtime_spine.canonical_content_refs.not_trust",
      source_area = "runtime_spine",
      implementation_path = "src/node/parts/runtime/p002/body.rs",
      verification_path = "src/node/parts/runtime/tests/m000/p000/body.rs",
      evidence_scope = "well-shaped payload identity remains separate from authority, policy, and resource admission",
    },
    {
      requirement_id = "molten.runtime_spine.canonical_content_refs.runtime_values",
      source_area = "runtime_spine",
      implementation_path = "src/runtime/turn/mod.rs",
      verification_path = "src/runtime/predicates/parts/mod/tests/m000/p002/body.rs",
      evidence_scope = "runtime values, messages, assertions, observations, events, turns, and snapshots derive canonical Preserves identities stable under replay",
    },
    {
      requirement_id = "molten.runtime_spine.canonical_content_refs.scoped_aliases",
      source_area = "runtime_spine",
      implementation_path = "src/octet/parts/gate/p009/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "Octet b3 aliases derive from checked BLAKE3 bytes while the runtime parser rejects b3 as canonical identity",
    },
    {
      requirement_id = "molten.runtime_spine.canonical_content_refs.shape",
      source_area = "runtime_spine",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "the shared ContentRef parser accepts only blake3 followed by sixty-four lowercase hexadecimal characters",
    },
  ],
  rejected_candidates = [
    {
      requirement_id = "molten.runtime_spine.canonical_content_refs.helper_construction",
      reason = "current production subsystems still construct BLAKE3 refs with local formatting helpers",
      counterexample_path = "src/fabric_transport/adapters.rs",
    },
    {
      requirement_id = "molten.runtime_spine.canonical_content_refs.no_ad_hoc_formatting",
      reason = "current production source still contains direct prefix formatting and manipulation outside the Preserves rail",
      counterexample_path = "src/fabric_transport/cross_process/iroh_shell.rs",
    },
  ],
  non_claims = [
    "universal-helper-only-construction",
    "all-ad-hoc-formatting-removed",
    "content-ref-as-trust",
    "complete-runtime-spine-coverage",
    "release-readiness",
    "whole-system-correctness",
  ],
}
in
manifest
