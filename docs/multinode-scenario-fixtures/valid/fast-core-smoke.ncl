let Contracts = import "../../multinode-scenario-contracts.ncl" in
{
  schema_id = Contracts.schema_id,
  schema_version = Contracts.schema_version,
  source_language = Contracts.source_language,
  fixture = {
    scenario_id = "fast-core-smoke",
    purpose = "review fast deterministic core evidence metadata",
    evidence_scope = "pure core and receipt validation evidence",
    topology_profile_id = "pairwise-transport",
    execution_profile_id = "fast",
    command_surface = "cargo nextest run --profile deterministic",
    expected_artifact_kinds = ["libtest", "junit"],
    topology_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000011",
    seed_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000012",
    fault_plan_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000013",
    receipt_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000014"],
    variance_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000015"],
    diagnostic_log_refs = ["blake3:0000000000000000000000000000000000000000000000000000000000000016"],
    unavailable_policy = "unavailable-is-deny",
    unsupported_claims_pass = false,
    caveats = ["logs-diagnostic-only", "fixture-evidence-only", "unsupported-is-not-pass"],
  },
} | Contracts.ScenarioFixtureExport
