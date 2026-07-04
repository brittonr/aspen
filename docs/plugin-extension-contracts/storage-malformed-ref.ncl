let PluginExtensionContract = import "contract.ncl" in
((import "storage.ncl") & {
  policy_refs | force = ["sha256:not-a-blake3-ref"],
}) | PluginExtensionContract
