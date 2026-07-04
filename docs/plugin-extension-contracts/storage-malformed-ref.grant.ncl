let PluginCapabilityGrant = import "grant.ncl" in
((import "storage.grant.ncl") & {
  manifest_ref | force = "sha256:not-a-blake3-ref",
}) | PluginCapabilityGrant
