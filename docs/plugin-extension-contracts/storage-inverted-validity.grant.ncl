let PluginCapabilityGrant = import "grant.ncl" in
let LaterTurn = 16 in
let EarlierTurn = 8 in
((import "storage.grant.ncl") & {
  attenuation.valid_from_turn | force = LaterTurn,
  attenuation.valid_until_turn | force = EarlierTurn,
}) | PluginCapabilityGrant
