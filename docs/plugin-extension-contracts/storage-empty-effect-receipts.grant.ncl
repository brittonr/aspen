let PluginCapabilityGrant = import "grant.ncl" in
((import "storage.grant.ncl") & {
  effect_receipt_refs | force = [],
}) | PluginCapabilityGrant
