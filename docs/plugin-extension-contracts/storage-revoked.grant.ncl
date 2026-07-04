let PluginCapabilityGrant = import "grant.ncl" in
((import "storage.grant.ncl") & {
  revocation_refs | force = ["blake3:3434343434343434343434343434343434343434343434343434343434343434"],
  revoked | force = true,
}) | PluginCapabilityGrant
