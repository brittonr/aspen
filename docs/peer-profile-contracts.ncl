# Typed Nickel contract for static peer profile exports.
# Runtime Rust consumes checked exported JSON; it does not evaluate Nickel.

let SchemaId = "molten.peer-profile.export.v1" in
let SchemaVersion = 1 in
let SourceLanguage = "nickel" in
let MaxStaticPeerProfiles = 32 in

let is_non_empty_string = fun value =>
  std.is_string value && std.string.length value > 0 in

let is_blake3_ref = fun value =>
  std.is_string value && std.string.is_match "^blake3:[0-9a-f]{64}$" value in

let distinct_strings = fun values =>
  std.array.length values == std.array.length (std.array.sort_dedup std.string.compare values) in

let array_of = fun predicate values =>
  std.is_array values && std.array.length values > 0 && std.array.all predicate values in

let is_peer_profile = fun profile =>
  std.is_record profile
  && is_blake3_ref profile.peer_ref
  && is_non_empty_string profile.endpoint
  && is_non_empty_string profile.topic
  && is_non_empty_string profile.scope
  && is_blake3_ref profile.resource_ref
  && array_of is_blake3_ref profile.bootstrap_refs
  && array_of is_blake3_ref profile.policy_refs
  && array_of is_blake3_ref profile.resource_refs in

let is_peer_profile_export = fun value =>
  std.is_record value
  && value.schema_id == SchemaId
  && value.schema_version == SchemaVersion
  && value.source_language == SourceLanguage
  && std.is_array value.profiles
  && std.array.length value.profiles > 0
  && std.array.length value.profiles <= MaxStaticPeerProfiles
  && std.array.all is_peer_profile value.profiles
  && distinct_strings (std.array.map (fun profile => profile.peer_ref) value.profiles) in

let PeerProfileExportContract = std.contract.from_predicate is_peer_profile_export in

{
  schema_id = SchemaId,
  schema_version = SchemaVersion,
  source_language = SourceLanguage,
  max_static_peer_profiles = MaxStaticPeerProfiles,
  peer_profile_export_contract = PeerProfileExportContract,
}
