let Contracts = import "../../multinode-scenario-contracts.ncl" in
{
  schema_id = Contracts.schema_id,
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  fixture = {
    scenario_id = "soak-pilot",
    purpose = "review constrained pilot and dogfood evidence metadata",
    evidence_scope = "pilot and readiness review evidence only",
    topology_profile_id = "control-quorum",
    execution_profile_id = "soak",
    command_surface = "nix build .#checks.x86_64-linux.dogfood-local-node",
    expected_artifact_kinds = ["prod-soak-run-v1", "operator-release-gate-receipt-v1"],
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
