let Template = import "../../profile-template.ncl" in
Template.make { object_storage_policy = {
  max_local_tier_bytes = 68719476737,
  minimum_verified_bytes_per_second = 1073741824, max_rebuild_seconds = 64,
  rebuild_path = "refetch-and-reverify-hot-chunks", at_capacity = "verified-remote-only-no-eviction",
  max_remote_connections = 1, endpoint_resolution = "fixed-numeric-at-admission",
  credential_refresh = "fixed-at-admission-no-refresh",
} }
