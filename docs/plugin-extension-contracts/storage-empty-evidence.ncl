let PluginExtensionContract = import "contract.ncl" in
((import "storage.ncl") & {
  supply_chain_refs | force = [],
}) | PluginExtensionContract
