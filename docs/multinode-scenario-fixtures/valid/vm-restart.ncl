let Contracts = import "../../multinode-scenario-contracts.ncl" in
{
  schema_id = Contracts.schema_id,
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  fixture = {
    scenario_id = "vm-restart",
    purpose = "review executable VM restart and rejoin workflow metadata",
    evidence_scope = "bounded restart, rejoin, and post-restart health evidence when host support is available",
    topology_profile_id = "restart-rejoin",
    execution_profile_id = "vm-restart",
    command_surface = "nix build .#checks.x86_64-linux.nixos-vm-restart",
    expected_artifact_kinds = ["nixos-vm-shard-run-v1", "node-startup-receipt-v1", "node-health-receipt-v1"],
    topology_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000061",
    seed_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000062",
    fault_plan_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000063",
    receipt_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000064"],
    variance_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000065"],
    diagnostic_log_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000066"],
    unavailable_policy = "unavailable-is-diagnostic-only",
    unsupported_claims_pass = false,
    caveats = ["logs-diagnostic-only", "fixture-evidence-only", "unsupported-is-not-pass"],
  },
} | Contracts.ScenarioFixtureExport
