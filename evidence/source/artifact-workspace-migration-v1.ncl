# Typed evidence for Molten's unified Artifact source migration.
let SourceRevision = "c932138d880ddf4c2967f4c024b489b5c0022bf1" in
let SourceRepository = "ssh://git@github.com/OnixResearch/onix-artifact.git" in
{
  schema_version = 1,
  receipt_type = "molten.artifact-source-migration.v1",
  status = "accepted",
  observed_date = "2026-08-06",
  source = {
    repository = SourceRepository,
    profile_repository = "https://github.com/OnixResearch/onix-artifact",
    revision = SourceRevision,
    nar_hash = "sha256-XGQLG60DNeY9FUYcOmn6cfYnhCIJzyqf+VW9yofDYFU=",
    source_archive_blake3 = "3878cdb892bfd4a8eac5779023ba32871d61bc3e8ec7c9ef5f7ab790a8acfeb9",
    license = "MIT OR Apache-2.0",
    packages = ["artifact-auth-core", "artifact-auth-ed25519", "artifact-binding-core", "artifact-transfer-core"],
  },
  consumer = {
    cargo_lock_source = "git+%{SourceRepository}?rev=%{SourceRevision}#%{SourceRevision}",
    packages = ["artifact-auth-core", "artifact-auth-ed25519", "artifact-binding-core"],
    transfer_package_resolved = false,
    binding_package_resolved = true,
    cargo_nix_policy_plans_aligned = true,
    single_nix_input = true,
  },
  parity = {
    core_blake3 = "839ea3e35429e2a1252020dd779b46ebd185b6911674036820da118d487d1d21",
    ed25519_blake3 = "7fd3987fe3ddc6d30efca37878952135f19905a00b52f349db237054be6b3633",
    rust_implementation_changed = false,
  },
  validation = {
    baseline_revision = "d69d4d734886ac40dada69cb0bda27985b0cd1de",
    baseline_core = "passed-4",
    baseline_source_check = "passed",
    baseline_cairn = "blocked-policy-schema",
    post_core = "passed",
    post_binding = "passed",
    formatting = "passed",
    clippy = "passed",
    source_admission_positive = "passed",
    source_admission_negative = "passed",
  },
  history = {
    repository = "https://git.onix.computer/z4JGYYW7WsesXUq7MXVdx16Fawu2f.git",
    revision = "799459346d5416fbd7b9f55840a7371441b55afa",
    receipt_blake3 = "a71619befdcef0c09fef0e45b82b87b3e786864d50471e0ca43c806b2e3aa1b4",
    status = "historical-only",
  },
  rollback = {
    automatic = false,
    requires_owning_tool_regeneration = true,
    requires_source_identity_review = true,
  },
  non_claims = ["artifact-source-correctness", "whole-molten-behavior-correctness", "forge-availability", "automatic-fallback", "runtime-or-release-authority", "release-readiness"],
  evidence = [
    "cairn/archive/2026-08-06-consume-artifact-auth-from-unified-workspace/evidence/source-migration-2026-08-06.md",
    "evidence/source/artifact-workspace-migration-v1.json",
    "evidence/radicle/artifact-auth-cutover-v1.json",
  ],
}
