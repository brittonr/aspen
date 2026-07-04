let Contracts = import "../../multinode-scenario-contracts.ncl" in
{
  schema_id = Contracts.schema_id,
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  fixture = {
    scenario_id = "vm-smoke",
    purpose = "review NixOS VM smoke topology metadata",
    evidence_scope = "platform integration smoke evidence when host support is available",
    topology_profile_id = "pairwise-transport",
    execution_profile_id = "vm-smoke",
    command_surface = "nix build .#checks.x86_64-linux.nixos-vm-multinode",
    expected_artifact_kinds = ["nixos-vm-test-run-v1", "nixos-vm-evidence-validation-v1"],
    topology_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000021",
    seed_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000022",
    fault_plan_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000023",
    receipt_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000024"],
    variance_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000025"],
    diagnostic_log_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000026"],
    unavailable_policy = "unavailable-is-diagnostic-only",
    unsupported_claims_pass = false,
    caveats = ["logs-diagnostic-only", "fixture-evidence-only", "unsupported-is-not-pass"],
  },
} | Contracts.ScenarioFixtureExport
