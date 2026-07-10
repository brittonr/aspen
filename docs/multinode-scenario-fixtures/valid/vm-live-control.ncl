let Contracts = import "../../multinode-scenario-contracts.ncl" in
{
  schema_id = Contracts.schema_id,
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  fixture = {
    scenario_id = "vm-live-control",
    purpose = "review executable VM live-control workflow metadata",
    evidence_scope = "bounded live-control send and receipt evidence when host support is available",
    topology_profile_id = "pairwise-transport",
    execution_profile_id = "vm-live-control",
    command_surface = "nix build .#checks.x86_64-linux.nixos-vm-live-control",
    expected_artifact_kinds = ["nixos-vm-shard-run-v1", "node-control-live-workflow-receipt-v1"],
    topology_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000041",
    seed_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000042",
    fault_plan_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000043",
    receipt_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000044"],
    variance_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000045"],
    diagnostic_log_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000046"],
    unavailable_policy = "unavailable-is-diagnostic-only",
    unsupported_claims_pass = false,
    caveats = ["logs-diagnostic-only", "fixture-evidence-only", "unsupported-is-not-pass"],
  },
} | Contracts.ScenarioFixtureExport
