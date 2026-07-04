let PluginExtensionContract = import "contract.ncl" in
((import "storage.ncl") & {
  extension_id | force = "storage",
}) | PluginExtensionContract
