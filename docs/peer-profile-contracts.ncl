# Typed Nickel contract for static peer profile exports.
# Runtime Rust consumes checked exported JSON; it does not evaluate Nickel.

let Prelude = import "nickel-contract-prelude.ncl" in
let SchemaId = "molten.peer-profile.export.v1" in
let SchemaVersion = 1 in
let SourceLanguage = "nickel" in
let MaxStaticPeerProfiles = 32 in

let is_non_empty_string = Prelude.is_non_empty_string in
let is_blake3_ref = Prelude.is_blake3_ref in
let array_of = Prelude.array_of in

let is_peer_profile = fun profile =>
  std.is_record profile
  && is_blake3_ref profile.peer_ref
  && is_non_empty_string profile.endpoint
  && is_non_empty_string profile.topic
  && is_non_empty_string profile.scope
  && is_blake3_ref profile.resource_ref
  && Prelude.non_empty_distinct_blake3_refs profile.bootstrap_refs
  && Prelude.non_empty_distinct_blake3_refs profile.policy_refs
  && Prelude.non_empty_distinct_blake3_refs profile.resource_refs in

let is_peer_profile_export = fun value =>
  std.is_record value
  && value.schema_id == SchemaId
  && value.schema_version == SchemaVersion
  && value.source_language == SourceLanguage
  && Prelude.is_non_empty_bounded_array MaxStaticPeerProfiles value.profiles
  && std.array.all is_peer_profile value.profiles
  && Prelude.bounded_distinct_by MaxStaticPeerProfiles (fun profile => profile.peer_ref) value.profiles in

let PeerProfileExportContract = std.contract.from_predicate is_peer_profile_export in

{
  schema_id = SchemaId,
  schema_version = SchemaVersion,
  source_language = SourceLanguage,
  max_static_peer_profiles = MaxStaticPeerProfiles,
  peer_profile_export_contract = PeerProfileExportContract,
}
