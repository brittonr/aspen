let Contracts = import "contracts.ncl" in
let CallbackBytes = 1048576 in
let DiagnosticBytes = 1048576 in
let MaterializedValueBytes = 262144 in
let InstanceCount = 16 in
let UnresolvedOperationCount = 256 in
let PortBindingCount = 64 in
let PolicyRefCount = 64 in
let MaterializedValueCount = 256 in
{
  local_live_pilot | Contracts.NativeHostProfile = {
    schema = "molten.system-extension.native-host-profile.v2",
    profile_id = "native-host-local-pilot-v2",
    execution_profile = "native-process",
    execution_port_profile = "bounded-process-live-v1",
    transport_profile = "iroh-native-extension-local-v1",
    claim_level = "local-live-materialized-values-pilot",
    alpn = "molten/system-extension/native/v2",
    framing = "preserves-packed-materialized-values-v2",
    environment_mode = "clear",
    process_per_callback = true,
    requires_materialized_values = true,
    bounds = {
      callback_input_bytes = CallbackBytes,
      callback_output_bytes = CallbackBytes,
      diagnostic_bytes = DiagnosticBytes,
      materialized_value_bytes = MaterializedValueBytes,
      instances = InstanceCount,
      unresolved_operations = UnresolvedOperationCount,
      port_bindings = PortBindingCount,
      policy_refs = PolicyRefCount,
      materialized_values = MaterializedValueCount,
    },
    ports = [
      "execution",
      "durability",
      "content",
      "transport",
      "time",
      "scheduling",
      "evidence",
    ],
    authorities = Contracts.RequiredAuthorities,
    lifecycle = Contracts.RequiredLifecycle,
    executable_evidence = Contracts.RequiredExecutableEvidence,
    non_claims = Contracts.RequiredNonClaims,
  },
}
