let Contracts = import "../../multinode-scenario-contracts.ncl" in
{
  schema_id = Contracts.schema_id,
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  fixture = {
    scenario_id = "vm-aggregate",
    purpose = "review executable VM aggregate receipt metadata",
    evidence_scope = "bounded aggregate evidence linking required shard receipts without promoting unavailable runs",
    topology_profile_id = "three-node-quorum",
    execution_profile_id = "vm-aggregate",
    command_surface = "nix build .#checks.x86_64-linux.nixos-vm-aggregate",
    expected_artifact_kinds = ["nixos-vm-multinode-aggregate-v1", "nixos-vm-evidence-manifest-v1"],
    topology_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000081",
    seed_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000082",
    fault_plan_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000083",
    receipt_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000084"],
    variance_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000085"],
    diagnostic_log_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000086"],
    unavailable_policy = "unavailable-is-diagnostic-only",
    unsupported_claims_pass = false,
    caveats = ["logs-diagnostic-only", "fixture-evidence-only", "unsupported-is-not-pass"],
  },
} | Contracts.ScenarioFixtureExport
