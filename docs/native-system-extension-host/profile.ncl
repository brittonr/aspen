let Contracts = import "contracts.ncl" in
let CallbackBytes = 1048576 in
let DiagnosticBytes = 1048576 in
let InstanceCount = 16 in
let UnresolvedOperationCount = 256 in
let PortBindingCount = 64 in
let PolicyRefCount = 64 in
{
  local_live_pilot | Contracts.NativeHostProfile = {
    schema = "molten.system-extension.native-host-profile.v1",
    profile_id = "native-host-local-pilot-v1",
    execution_profile = "native-process",
    execution_port_profile = "bounded-process-live-v1",
    transport_profile = "iroh-native-extension-local-v1",
    claim_level = "local-live-pilot",
    alpn = "molten/system-extension/native/v1",
    framing = "preserves-packed-single-frame-v1",
    environment_mode = "clear",
    process_per_callback = true,
    bounds = {
      callback_input_bytes = CallbackBytes,
      callback_output_bytes = CallbackBytes,
      diagnostic_bytes = DiagnosticBytes,
      instances = InstanceCount,
      unresolved_operations = UnresolvedOperationCount,
      port_bindings = PortBindingCount,
      policy_refs = PolicyRefCount,
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
