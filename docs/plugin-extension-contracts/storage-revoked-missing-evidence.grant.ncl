let PluginCapabilityGrant = import "grant.ncl" in
((import "storage.grant.ncl") & {
  revoked | force = true,
  revocation_refs | force = [],
}) | PluginCapabilityGrant
