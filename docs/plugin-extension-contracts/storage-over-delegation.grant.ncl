let PluginCapabilityGrant = import "grant.ncl" in
let DepthBeyondGrantMaximum = 3 in
((import "storage.grant.ncl") & {
  attenuation.current_delegation_depth | force = DepthBeyondGrantMaximum,
}) | PluginCapabilityGrant
