# Shared Nickel contracts for declarative multinode scenario fixtures.
# Runtime Rust consumes checked exported data; it does not evaluate Nickel.

let Prelude = import "nickel-contract-prelude.ncl" in
let SchemaId = "molten.testing.multinode.scenario-fixture.v1" in
let SchemaVersion = 1 in
let SourceLanguage = "nickel" in

let RequiredUnavailablePolicies = [
  "unavailable-is-deny",
  "unavailable-is-diagnostic-only",
] in

let RequiredEvidenceCaveats = [
  "logs-diagnostic-only",
  "fixture-evidence-only",
  "unsupported-is-not-pass",
] in

let ProtocolCommandSurface = "cargo test --lib distributed" in
let ProtocolArtifactKinds = [
  "distributed-test-run-v1",
  "distributed-fault-plan-v1",
] in

let is_non_empty_string = Prelude.is_non_empty_string in
let is_blake3_ref = Prelude.is_blake3_ref in
let is_boolean = fun value => std.is_bool value in
let is_exact_schema_id = Prelude.is_exact_string SchemaId in
let is_exact_schema_version = Prelude.is_exact_integer SchemaVersion in
let is_exact_source_language = Prelude.is_exact_string SourceLanguage in
let array_of = Prelude.array_of in
let is_one_of = Prelude.is_one_of in
let has_all = Prelude.has_all in

let matches_profile_surface = fun fixture =>
  if fixture.execution_profile_id == "protocol" then
    fixture.command_surface == ProtocolCommandSurface
    && fixture.expected_artifact_kinds == ProtocolArtifactKinds
  else
    true in

let is_scenario_fixture = fun fixture =>
  std.is_record fixture
  && is_non_empty_string fixture.scenario_id
  && is_non_empty_string fixture.purpose
  && is_non_empty_string fixture.evidence_scope
  && is_non_empty_string fixture.topology_profile_id
  && is_non_empty_string fixture.execution_profile_id
  && is_non_empty_string fixture.command_surface
  && array_of is_non_empty_string fixture.expected_artifact_kinds
  && is_blake3_ref fixture.topology_ref
  && is_blake3_ref fixture.seed_ref
  && is_blake3_ref fixture.fault_plan_ref
  && array_of is_blake3_ref fixture.receipt_refs
  && array_of is_blake3_ref fixture.variance_refs
  && array_of is_blake3_ref fixture.diagnostic_log_refs
  && is_one_of RequiredUnavailablePolicies fixture.unavailable_policy
  && is_boolean fixture.unsupported_claims_pass
  && fixture.unsupported_claims_pass == false
  && matches_profile_surface fixture
  && array_of is_non_empty_string fixture.caveats
  && has_all fixture.caveats RequiredEvidenceCaveats in

let is_scenario_fixture_export = fun value =>
  std.is_record value
  && is_exact_schema_id value.schema_id
  && is_exact_schema_version value.schema_version
  && is_exact_source_language value.source_language
  && is_scenario_fixture value.fixture in

let NonEmptyStringContract = std.contract.from_predicate is_non_empty_string in
let Blake3RefContract = std.contract.from_predicate is_blake3_ref in
let ExactSchemaIdContract = std.contract.from_predicate is_exact_schema_id in
let ExactSchemaVersionContract = std.contract.from_predicate is_exact_schema_version in
let ExactSourceLanguageContract = std.contract.from_predicate is_exact_source_language in
let ScenarioFixtureContract = std.contract.from_predicate is_scenario_fixture in
let ScenarioFixtureExportContract = std.contract.from_predicate is_scenario_fixture_export in

{
  schema_id = SchemaId,
  schema_version = SchemaVersion,
  source_language = SourceLanguage,
  vocabulary = {
    unavailable_policies = RequiredUnavailablePolicies,
    required_evidence_caveats = RequiredEvidenceCaveats,
    protocol_command_surface = ProtocolCommandSurface,
    protocol_artifact_kinds = ProtocolArtifactKinds,
  },
  contracts = {
    Blake3Ref = Blake3RefContract,
    ExactSchemaId = ExactSchemaIdContract,
    ExactSchemaVersion = ExactSchemaVersionContract,
    ExactSourceLanguage = ExactSourceLanguageContract,
    NonEmptyString = NonEmptyStringContract,
    ScenarioFixture = ScenarioFixtureContract,
  },
  ScenarioFixtureExport = ScenarioFixtureExportContract,
}
