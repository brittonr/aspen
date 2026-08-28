let PositiveNatural = fun value => std.is_number value && value > 0 in
let AuthoritySet = [
  "execution",
  "resources",
  "supervision",
  "evidence",
]
in
let LifecycleSet = [
  "install",
  "start",
  "request",
  "status",
  "recover",
  "drain",
  "stop",
  "remove",
]
in
let EvidenceSet = [
  "executable-bytes",
  "artifact-kind",
  "target",
  "dependency-closure",
  "materialization",
  "provenance",
  "source-gate",
  "policy",
  "authority",
  "resources",
  "execution-profile",
]
in
let NonClaimSet = [
  "does-not-prove-sandboxing",
  "does-not-prove-hermeticity",
  "does-not-prove-executable-trust",
  "does-not-prove-callback-correctness",
  "does-not-prove-effect-success",
  "does-not-prove-value-meaning",
  "does-not-prove-value-durability",
  "does-not-prove-transport-delivery",
  "does-not-prove-distributed-availability",
  "does-not-prove-production-readiness",
]
in
let ContainsAll = fun required actual =>
  std.array.all (fun item => std.array.elem item actual) required
in
let IsProfile = fun value =>
  std.is_record value
  && value.schema == "molten.system-extension.native-host-profile.v2"
  && value.execution_profile == "native-process"
  && value.claim_level == "local-live-materialized-values-pilot"
  && value.alpn == "molten/system-extension/native/v2"
  && value.framing == "preserves-packed-materialized-values-v2"
  && value.environment_mode == "clear"
  && value.process_per_callback == true
  && value.requires_materialized_values == true
  && PositiveNatural value.bounds.callback_input_bytes
  && PositiveNatural value.bounds.callback_output_bytes
  && PositiveNatural value.bounds.diagnostic_bytes
  && PositiveNatural value.bounds.materialized_value_bytes
  && PositiveNatural value.bounds.instances
  && PositiveNatural value.bounds.unresolved_operations
  && PositiveNatural value.bounds.port_bindings
  && PositiveNatural value.bounds.policy_refs
  && PositiveNatural value.bounds.materialized_values
  && std.is_array value.authorities
  && ContainsAll AuthoritySet value.authorities
  && std.array.length value.authorities == std.array.length AuthoritySet
  && std.is_array value.lifecycle
  && ContainsAll LifecycleSet value.lifecycle
  && std.array.length value.lifecycle == std.array.length LifecycleSet
  && std.is_array value.executable_evidence
  && ContainsAll EvidenceSet value.executable_evidence
  && std.array.length value.executable_evidence == std.array.length EvidenceSet
  && std.is_array value.non_claims
  && ContainsAll NonClaimSet value.non_claims
  && std.array.length value.non_claims == std.array.length NonClaimSet
in
{
  NativeHostProfile = std.contract.from_predicate IsProfile,
  RequiredAuthorities = AuthoritySet,
  RequiredLifecycle = LifecycleSet,
  RequiredExecutableEvidence = EvidenceSet,
  RequiredNonClaims = NonClaimSet,
}
