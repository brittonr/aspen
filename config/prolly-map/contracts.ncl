let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let ExpectedFormatVersion = 1 in
let ExpectedMinimumNodeBytes = 256 in
let ExpectedTargetNodeBytes = 1024 in
let ExpectedMaximumNodeBytes = 4096 in
let ExpectedMinimumFanout = 2 in
let ExpectedTargetFanout = 4 in
let ExpectedMaximumFanout = 8 in
let ExpectedMaximumKeyBytes = 64 in
let ExpectedMaximumValueBytes = 1024 in
let ExpectedMaximumEntries = 4096 in
let ExpectedMaximumTreeHeight = 16 in
let ExpectedMaximumDiffRecords = 8192 in
let ExpectedMaximumGraphFacts = 16384 in
let Blake3Ref =
  Prelude.contract (fun value =>
    std.is_string value && std.string.is_match "^blake3:[0-9a-f]{64}$" value
  )
in
let Limits = {
  max_key_bytes | Number,
  max_value_bytes | Number,
  max_entries | Number,
  max_tree_height | Number,
  max_diff_records | Number,
  max_graph_facts | Number,
}
in
let ProfileShape = {
  schema | Prelude.contract (Prelude.is_exact_string "molten.prolly-map-profile.v1"),
  format | Prelude.contract (Prelude.is_exact_string "prolly-semantic-map-v1"),
  format_version | Number,
  profile_ref | Blake3Ref,
  key_codec | Prelude.contracts.NonEmptyString,
  value_codec | Prelude.contracts.NonEmptyString,
  comparison | Prelude.contracts.NonEmptyString,
  node_codec | Prelude.contracts.NonEmptyString,
  boundary_domain | Prelude.contracts.NonEmptyString,
  boundary_seed_ref | Blake3Ref,
  size_accounting | Prelude.contracts.NonEmptyString,
  min_node_bytes | Number,
  target_node_bytes | Number,
  max_node_bytes | Number,
  min_fanout | Number,
  target_fanout | Number,
  max_fanout | Number,
  profile_domain | Prelude.contracts.NonEmptyString,
  leaf_domain | Prelude.contracts.NonEmptyString,
  internal_domain | Prelude.contracts.NonEmptyString,
  root_domain | Prelude.contracts.NonEmptyString,
  limits | Limits,
  claims_collision_impossibility | Bool,
  claims_database_correctness | Bool,
  owns_branch_authority | Bool,
  owns_effect_release | Bool,
  owns_gc_deletion | Bool,
  extraction_approved | Bool,
}
in
let invariants = fun profile =>
  profile.format_version == ExpectedFormatVersion
  && profile.profile_ref == "blake3:820d0424eac0ce727d80750e485dcaac320137baa5b4ef21d0d67708ca8a41d5"
  && profile.key_codec == "canonical-bytes-v1"
  && profile.value_codec == "canonical-bytes-v1"
  && profile.comparison == "unsigned-lexicographic-v1"
  && profile.node_codec == "molten-prolly-node-binary-v1"
  && profile.boundary_domain == "molten-prolly-boundary:v1"
  && profile.boundary_seed_ref == "blake3:4da8a4af528826c160a956f28ad0b8cce119c2636ea8a94b48b99e6596805261"
  && profile.size_accounting == "exact-encoded-bytes-v1"
  && profile.min_node_bytes == ExpectedMinimumNodeBytes
  && profile.target_node_bytes == ExpectedTargetNodeBytes
  && profile.max_node_bytes == ExpectedMaximumNodeBytes
  && profile.min_fanout == ExpectedMinimumFanout
  && profile.target_fanout == ExpectedTargetFanout
  && profile.max_fanout == ExpectedMaximumFanout
  && profile.profile_domain == "molten-prolly-profile:v1"
  && profile.leaf_domain == "molten-prolly-leaf:v1"
  && profile.internal_domain == "molten-prolly-internal:v1"
  && profile.root_domain == "molten-prolly-root:v1"
  && profile.limits == {
    max_key_bytes = ExpectedMaximumKeyBytes,
    max_value_bytes = ExpectedMaximumValueBytes,
    max_entries = ExpectedMaximumEntries,
    max_tree_height = ExpectedMaximumTreeHeight,
    max_diff_records = ExpectedMaximumDiffRecords,
    max_graph_facts = ExpectedMaximumGraphFacts,
  }
  && profile.claims_collision_impossibility == false
  && profile.claims_database_correctness == false
  && profile.owns_branch_authority == false
  && profile.owns_effect_release == false
  && profile.owns_gc_deletion == false
  && profile.extraction_approved == false
in
{
  Profile = std.contract.Sequence [ProfileShape, Prelude.contract invariants],
  Limits = Limits,
}
