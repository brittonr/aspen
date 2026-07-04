let Contracts = import "../../multinode-scenario-contracts.ncl" in
{
  schema_id = Contracts.schema_id,
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  fixture = {
    scenario_id = "missing-command-surface",
    purpose = "negative fixture: command surface empty",
    evidence_scope = "negative fixture evidence",
    topology_profile_id = "pairwise-transport",
    execution_profile_id = "protocol",
    command_surface = "",
    expected_artifact_kinds = ["distributed-test-run-v1"],
    topology_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000001",
    seed_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000002",
    fault_plan_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000003",
    receipt_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000004"],
    variance_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000005"],
    diagnostic_log_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000006"],
    unavailable_policy = "unavailable-is-deny",
    unsupported_claims_pass = false,
    caveats = ["logs-diagnostic-only", "fixture-evidence-only", "unsupported-is-not-pass"],
  },
} | Contracts.ScenarioFixtureExport
