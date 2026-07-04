let PluginExtensionContract = import "contract.ncl" in
((import "storage.ncl") & {
  profile | force = "root-authority",
}) | PluginExtensionContract
