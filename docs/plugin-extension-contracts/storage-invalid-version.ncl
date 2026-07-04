let PluginExtensionContract = import "contract.ncl" in
((import "storage.ncl") & {
  version | force = "latest",
}) | PluginExtensionContract
