let Contracts = import "../../multinode-scenario-contracts.ncl" in
{
  schema_id = Contracts.schema_id,
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  fixture = {
    scenario_id = "vm-service-job",
    purpose = "review executable VM service and job workflow metadata",
    evidence_scope = "bounded service supervisor and job lifecycle evidence when host support is available",
    topology_profile_id = "pairwise-transport",
    execution_profile_id = "vm-service-job",
    command_surface = "nix build .#checks.x86_64-linux.nixos-vm-service-job",
    expected_artifact_kinds = ["nixos-vm-shard-run-v1", "service-lifecycle-receipt-v1", "job-execution-receipt-v1"],
    topology_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000051",
    seed_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000052",
    fault_plan_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000053",
    receipt_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000054"],
    variance_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000055"],
    diagnostic_log_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000056"],
    unavailable_policy = "unavailable-is-diagnostic-only",
    unsupported_claims_pass = false,
    caveats = ["logs-diagnostic-only", "fixture-evidence-only", "unsupported-is-not-pass"],
  },
} | Contracts.ScenarioFixtureExport
