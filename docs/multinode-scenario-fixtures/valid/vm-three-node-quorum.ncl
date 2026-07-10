let Contracts = import "../../multinode-scenario-contracts.ncl" in
{
  schema_id = Contracts.schema_id,
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  fixture = {
    scenario_id = "vm-three-node-quorum",
    purpose = "review executable VM three-node quorum platform metadata",
    evidence_scope = "bounded quorum placement and platform receipt evidence when host support is available",
    topology_profile_id = "three-node-quorum",
    execution_profile_id = "vm-three-node-quorum",
    command_surface = "nix build .#checks.x86_64-linux.nixos-vm-three-node-quorum",
    expected_artifact_kinds = ["nixos-vm-shard-run-v1", "raft-commit-receipt-v1", "consensus-engine-receipt-v1"],
    topology_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000071",
    seed_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000072",
    fault_plan_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000073",
    receipt_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000074"],
    variance_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000075"],
    diagnostic_log_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000076"],
    unavailable_policy = "unavailable-is-diagnostic-only",
    unsupported_claims_pass = false,
    caveats = ["logs-diagnostic-only", "fixture-evidence-only", "unsupported-is-not-pass"],
  },
} | Contracts.ScenarioFixtureExport
