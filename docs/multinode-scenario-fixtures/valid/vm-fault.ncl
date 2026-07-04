let Contracts = import "../../multinode-scenario-contracts.ncl" in
{
  schema_id = Contracts.schema_id,
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  fixture = {
    scenario_id = "vm-fault",
    purpose = "review executable VM fault metadata",
    evidence_scope = "bounded executable platform fault evidence when host support is available",
    topology_profile_id = "restart-rejoin",
    execution_profile_id = "vm-fault",
    command_surface = "nix build .#checks.x86_64-linux.nixos-vm-multinode",
    expected_artifact_kinds = ["nixos-vm-fault-receipt-v1"],
    topology_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000031",
    seed_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000032",
    fault_plan_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000033",
    receipt_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000034"],
    variance_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000035"],
    diagnostic_log_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000036"],
    unavailable_policy = "unavailable-is-diagnostic-only",
    unsupported_claims_pass = false,
    caveats = ["logs-diagnostic-only", "fixture-evidence-only", "unsupported-is-not-pass"],
  },
} | Contracts.ScenarioFixtureExport
