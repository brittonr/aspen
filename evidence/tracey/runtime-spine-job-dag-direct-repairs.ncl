# Typed direct-evidence manifest for the runtime-spine blob-ref job batch.
# r[impl molten.project.runtime_spine_job_dag_tracey.direct_repairs]
# r[impl molten.project.runtime_spine_job_dag_tracey.exact_manifest]
# r[impl molten.project.runtime_spine_job_dag_tracey.non_claims]
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
  schema = "molten.runtime-spine-job-dag-tracey-direct-repairs.v1",
  observed_date = "2026-08-10",
  base_revision = "53f8f700f6fbd8facac379fcf50a7d4fe42b4a6c",
  accepted_specification = "cairn/specs/runtime-spine/spec.md",
  baseline_before = 1965,
  baseline_after = 1956,
  runtime_spine_before = 428,
  runtime_spine_after = 419,
  candidate_count = 12,
  repairs = [
    {
      requirement_id = "molten.blob_ref_jobs.content_verification",
      source_area = "blob_ref_jobs",
      implementation_path = "src/job/parts/dag/p014/body.rs",
      verification_path = "src/job/parts/dag/tests/m000/p000/body.rs",
      evidence_scope = "manifest read and verification before handler execution with missing-content denial",
    },
    {
      requirement_id = "molten.blob_ref_jobs.local_tests",
      source_area = "blob_ref_jobs",
      implementation_path = "src/job/parts/dag/p003/body.rs",
      verification_path = "src/job/parts/dag/tests/m000/p000/body.rs",
      evidence_scope = "positive local submit-fetch-verify-run-complete test and negative pre-run denial tests",
    },
    {
      requirement_id = "molten.blob_ref_jobs.local_worker",
      source_area = "blob_ref_jobs",
      implementation_path = "src/job/parts/dag/p003/body.rs",
      verification_path = "src/job/parts/dag/tests/m000/p000/body.rs",
      evidence_scope = "deterministic local fetch-verify-handler-output worker path",
    },
    {
      requirement_id = "molten.blob_ref_jobs.no_inline_large_bytes",
      source_area = "blob_ref_jobs",
      implementation_path = "src/job/parts/dag/p021/body.rs",
      verification_path = "src/job/parts/dag/tests/m000/p000/body.rs",
      evidence_scope = "inline executable, input, and dataset token denial before execution",
    },
    {
      requirement_id = "molten.blob_ref_jobs.payload_model",
      source_area = "blob_ref_jobs",
      implementation_path = "src/job/parts/dag/p003/body.rs",
      verification_path = "src/job/parts/dag/tests/m000/p000/body.rs",
      evidence_scope = "canonical content-ref-only job submission artifact",
    },
    {
      requirement_id = "molten.blob_ref_jobs.property_tests",
      source_area = "blob_ref_jobs",
      implementation_path = "src/job/parts/dag/p003/body.rs",
      verification_path = "src/job/parts/dag/tests/m000/p004/body.rs",
      evidence_scope = "bounded Hegel inline-token denial, verification-before-run, pin, and cleanup coverage",
    },
    {
      requirement_id = "molten.blob_ref_jobs.provenance_policy",
      source_area = "blob_ref_jobs",
      implementation_path = "src/job/parts/dag/p013/body.rs",
      verification_path = "src/job/parts/dag/tests/m000/p000/body.rs",
      evidence_scope = "explicit policy, provenance, and effect refs required for passing execution evidence",
    },
    {
      requirement_id = "molten.blob_ref_jobs.receipts",
      source_area = "blob_ref_jobs",
      implementation_path = "src/job/parts/dag/p014/body.rs",
      verification_path = "src/job/parts/dag/tests/m000/p000/body.rs",
      evidence_scope = "canonical pass and deny receipts bind submission, fetch, verification, pins, cleanup, and outputs",
    },
    {
      requirement_id = "molten.blob_ref_jobs.retention_pins",
      source_area = "blob_ref_jobs",
      implementation_path = "src/job/parts/dag/p003/body.rs",
      verification_path = "src/job/parts/dag/tests/m000/p000/body.rs",
      evidence_scope = "active content pinning and cleanup receipt orchestration",
    },
  ],
  rejected_candidates = [
    "molten.blob_ref_jobs.job_dag_integration",
    "molten.blob_ref_jobs.replay_integration",
    "molten.blob_ref_jobs.status_assertions",
  ],
  non_claims = [
    "blob-ref-replay-identity",
    "complete-blob-ref-status-transition-coverage",
    "true-job-dag-integration",
    "complete-runtime-spine-coverage",
    "release-readiness",
    "whole-system-correctness",
  ],
}
in
manifest
