let PluginExtensionContract = import "contract.ncl" in
let Contract = import "storage.ncl" in
(Contract & {
  hostcalls | force = Contract.hostcalls @ Contract.hostcalls,
}) | PluginExtensionContract
