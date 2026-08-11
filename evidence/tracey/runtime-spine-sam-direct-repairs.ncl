# Typed direct-evidence manifest for the runtime-spine SAM service batch.
# r[impl molten.project.runtime_spine_sam_tracey.direct_repairs]
# r[impl molten.project.runtime_spine_sam_tracey.exact_manifest]
# r[impl molten.project.runtime_spine_sam_tracey.non_claims]
let Repair = {
  requirement_id | String,
  source_area | String,
  implementation_path | String,
  verification_path | String,
  evidence_scope | String,
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
  rejected_candidates | Array String,
  non_claims | Array String,
}
in
let manifest | Manifest = {
  schema = "molten.runtime-spine-sam-tracey-direct-repairs.v1",
  observed_date = "2026-08-10",
  base_revision = "80fbe0a87ef7b8248b90ef4357eed152fe2cc037",
  accepted_specification = "cairn/specs/runtime-spine/spec.md",
  baseline_before = 1956,
  baseline_after = 1943,
  runtime_spine_before = 419,
  runtime_spine_after = 406,
  candidate_count = 13,
  repairs = [
    {
      requirement_id = "molten.sam_service_demand_runtime.spec.admitted_demand_start",
      source_area = "sam_service_demand_runtime",
      implementation_path = "src/service/parts/runtime/p001/body.rs",
      verification_path = "src/service/parts/runtime/p004/body.rs",
      evidence_scope = "canonical demand startup after explicit authority, policy, resource, effect-profile, and source-gate admission",
    },
    {
      requirement_id = "molten.sam_service_demand_runtime.spec.dependency_resolution",
      source_area = "sam_service_demand_runtime",
      implementation_path = "src/service/parts/runtime/p003/body.rs",
      verification_path = "src/service/parts/runtime/p004/body.rs",
      evidence_scope = "bounded deterministic dependency ordering with missing-dependency waits and cycle denial",
    },
    {
      requirement_id = "molten.sam_service_demand_runtime.spec.owned_assertion_replay",
      source_area = "sam_service_demand_runtime",
      implementation_path = "src/service/parts/runtime/p002/body.rs",
      verification_path = "src/service/parts/runtime/p004/body.rs",
      evidence_scope = "service-owned status and readiness artifacts bound to canonical replay identities",
    },
    {
      requirement_id = "molten.sam_service_records_ledger.spec.canonical_records",
      source_area = "sam_service_records_ledger",
      implementation_path = "src/service/parts/records/p002/body.rs",
      verification_path = "src/service/parts/records/tests/m000/p001/body.rs",
      evidence_scope = "typed canonical service records, stable BLAKE3 references, parsers, and ledger classification",
    },
    {
      requirement_id = "molten.sam_service_records_ledger.spec.catalog_redaction",
      source_area = "sam_service_records_ledger",
      implementation_path = "src/service/parts/records/p002/body.rs",
      verification_path = "src/service/parts/records/tests/m000/p001/body.rs",
      evidence_scope = "classified service artifacts with redacted summaries that cannot substitute for canonical evidence",
    },
    {
      requirement_id = "molten.sam_service_records_ledger.spec.explicit_boundaries",
      source_area = "sam_service_records_ledger",
      implementation_path = "src/service/parts/records/p001/body.rs",
      verification_path = "src/service/parts/records/tests/m000/p000/body.rs",
      evidence_scope = "service manifests require explicit owner authority, policy, resource, and effect-profile refs",
    },
    {
      requirement_id = "molten.sam_service_supervision.spec.cleanup",
      source_area = "sam_service_supervision",
      implementation_path = "src/service/parts/supervision/p003/body.rs",
      verification_path = "src/service/parts/supervision/tests/m000/p000/body.rs",
      evidence_scope = "termination and revocation cleanup retracts indexed service-owned state and emits canonical receipts",
    },
    {
      requirement_id = "molten.sam_service_supervision.spec.demand_start",
      source_area = "sam_service_supervision",
      implementation_path = "src/service/parts/runtime/p001/body.rs",
      verification_path = "src/service/parts/runtime/p004/body.rs",
      evidence_scope = "matching demand starts only after explicit startup evidence passes and emits no readiness on denial",
    },
    {
      requirement_id = "molten.sam_service_supervision.spec.supervision",
      source_area = "sam_service_supervision",
      implementation_path = "src/service/parts/supervision/p001/body.rs",
      verification_path = "src/service/parts/supervision/tests/m000/p000/body.rs",
      evidence_scope = "canonical logical links, monitor notifications, restart decisions, failure propagation, and deterministic replay",
    },
    {
      requirement_id = "molten.sam_service_supervision_cleanup.spec.bounded_restart",
      source_area = "sam_service_supervision_cleanup",
      implementation_path = "src/service/parts/supervision/p003/body.rs",
      verification_path = "src/service/parts/supervision/tests/m000/p000/body.rs",
      evidence_scope = "restart decisions bind policy, authority, resource, logical budget, lifecycle refs, and bounded denial",
    },
    {
      requirement_id = "molten.sam_service_supervision_cleanup.spec.cleanup_replay_retention",
      source_area = "sam_service_supervision_cleanup",
      implementation_path = "src/service/parts/supervision/p003/body.rs",
      verification_path = "src/service/parts/supervision/tests/m000/p000/body.rs",
      evidence_scope = "cleanup receipts bind lifecycle, ownership, revocation, retraction, resource, and retention inputs for replay",
    },
    {
      requirement_id = "molten.sam_service_supervision_cleanup.spec.logical_supervision",
      source_area = "sam_service_supervision_cleanup",
      implementation_path = "src/service/parts/supervision/p001/body.rs",
      verification_path = "src/service/parts/supervision/tests/m000/p000/body.rs",
      evidence_scope = "logical supervision records and receipts reject OS parentage as pass evidence",
    },
    {
      requirement_id = "molten.sam_service_supervision_cleanup.spec.owned_cleanup",
      source_area = "sam_service_supervision_cleanup",
      implementation_path = "src/service/parts/supervision/p003/body.rs",
      verification_path = "src/service/parts/supervision/tests/m000/p000/body.rs",
      evidence_scope = "owned assertions, observers, live refs, exposed refs, and pending effects retract while foreign claims deny",
    },
  ],
  rejected_candidates = [],
  non_claims = [
    "complete-runtime-spine-coverage",
    "ambient-service-authority",
    "operating-system-parentage-as-supervision",
    "retention-policy-bypass",
    "release-readiness",
    "whole-system-correctness",
  ],
}
in
manifest
