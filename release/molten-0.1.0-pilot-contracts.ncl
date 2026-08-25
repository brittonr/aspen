# Typed review contract for the Molten 0.1.0 limited-pilot manifest.

let Prelude = import "../docs/nickel-contract-prelude.ncl" in
let SchemaId = "molten.release.limited-pilot.v1" in
let SchemaVersion = 1 in
let is_ref = Prelude.is_blake3_ref in
let is_git_identity = fun value => std.string.is_match "^[0-9a-f]{40}$" value in
let is_non_empty = Prelude.is_non_empty_string in
let refs_are_valid = fun evidence =>
  is_ref evidence.source
  && is_ref evidence.rust
  && is_ref evidence.nextest
  && is_ref evidence.nix
  && is_ref evidence.cairn
  && is_ref evidence.octet
  && is_ref evidence.dogfood_report
  && is_ref evidence.dogfood_verify
  && is_ref evidence.bundle_verify
  && is_ref evidence.promotion
  && is_ref evidence.signed_promotion
  && is_ref evidence.export_verify
  && is_ref evidence.vm_test
  && is_ref evidence.vm_soak
  && is_ref evidence.vm_validation
  && is_ref evidence.vm_manifest
  && is_ref evidence.vm_network_unavailable
  && is_ref evidence.production_profile
  && is_ref evidence.production_profile_policy
  && is_ref evidence.release_profile_validation
  && is_ref evidence.pilot_decision
  && is_ref evidence.candidate_gate
  && is_ref evidence.missing_candidate_deny
  && is_ref evidence.mixed_source_diagnostic in
let is_manifest = fun value =>
  std.is_record value
  && value.schema == SchemaId
  && value.schema_version == SchemaVersion
  && value.release_kind == "limited-internal-pilot"
  && value.publication_status == "verified"
  && value.version == "0.1.0"
  && is_git_identity value.candidate.commit
  && is_git_identity value.candidate.tree
  && is_ref value.candidate.source_ref
  && value.candidate.source_ref == value.evidence.source
  && refs_are_valid value.evidence
  && Prelude.array_of is_non_empty value.scope.allowed_workloads
  && Prelude.array_of is_non_empty value.scope.denied_workloads
  && Prelude.array_of is_non_empty value.scope.rollback_triggers
  && Prelude.array_of is_non_empty value.scope.stop_conditions
  && Prelude.array_of is_non_empty value.caveats
  && Prelude.array_of is_non_empty value.non_claims
  && is_non_empty value.outputs.nextest
  && is_non_empty value.outputs.dogfood
  && is_non_empty value.outputs.vm
  && is_ref value.outputs.release_archive_ref in
let ManifestContract = std.contract.from_predicate is_manifest in
{
  schema_id = SchemaId,
  schema_version = SchemaVersion,
  Manifest = ManifestContract,
}
