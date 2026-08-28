let PositiveNatural = fun value => std.is_number value && value > 0 in
let RequiredClaims = [
  "does-not-prove-sandboxing",
  "does-not-prove-hermeticity",
  "does-not-prove-executable-trust",
  "does-not-prove-child-correctness",
  "does-not-prove-network-isolation",
  "does-not-prove-platform-equivalence",
  "does-not-prove-application-success",
  "does-not-prove-release-readiness",
]
in
let IsProfile = fun value =>
  std.is_record value
  && value.schema == "molten.fabric.execution-profile.v1"
  && std.array.elem value.kind ["live-bounded-process", "deterministic-simulation"]
  && std.array.elem value.platform ["unix-process-group", "direct-child-only"]
  && value.environment_mode == "clear"
  && value.invocation_mode == "direct"
  && value.executable_resolution == "exact-artifact"
  && value.workspace_mode == "capability-root"
  && value.component.repository == "https://git.onix.computer/z2CpqLFpdP36fZXYUK5ZNWxMibpCo.git"
  && value.component.revision == "29dac88ecded94457572db3fdfaaaab95fa91525"
  && value.component.license == "AGPL-3.0-or-later"
  && value.component.package == "bounded-exec"
  && std.is_array value.termination_scopes
  && std.array.length value.termination_scopes > 0
  && std.array.all (fun scope => std.array.elem scope ["direct-child", "process-group"]) value.termination_scopes
  && (
    value.platform == "unix-process-group"
    || !std.array.elem "process-group" value.termination_scopes
  )
  && PositiveNatural value.bounds.timeout_ms
  && PositiveNatural value.bounds.stdin_bytes
  && PositiveNatural value.bounds.stdout_bytes
  && PositiveNatural value.bounds.stderr_bytes
  && PositiveNatural value.bounds.poll_interval_ms
  && PositiveNatural value.bounds.teardown_timeout_ms
  && PositiveNatural value.bounds.arguments
  && PositiveNatural value.bounds.argument_bytes
  && PositiveNatural value.bounds.environment_entries
  && PositiveNatural value.bounds.environment_name_bytes
  && PositiveNatural value.bounds.environment_value_bytes
  && PositiveNatural value.bounds.concurrency_units
  && PositiveNatural value.bounds.queue_units
  && value.bounds.poll_interval_ms <= value.bounds.timeout_ms
  && std.is_array value.non_claims
  && std.array.length value.non_claims == std.array.length RequiredClaims
  && std.array.all (fun claim => std.array.elem claim value.non_claims) RequiredClaims
in
{
  ExecutionProfile = std.contract.from_predicate IsProfile,
  RequiredNonClaims = RequiredClaims,
}
