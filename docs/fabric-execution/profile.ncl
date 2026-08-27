let Contracts = import "contracts.ncl" in
let TimeoutMilliseconds = 30000 in
let StreamBytes = 1048576 in
let PollIntervalMilliseconds = 10 in
let TeardownTimeoutMilliseconds = 5000 in
let ArgumentCount = 64 in
let ArgumentBytes = 4096 in
let EnvironmentEntryCount = 64 in
let EnvironmentNameBytes = 128 in
let EnvironmentValueBytes = 4096 in
let ConcurrencyUnits = 1 in
let QueueUnits = 64 in
let Component = {
  repository = "https://git.onix.computer/z2CpqLFpdP36fZXYUK5ZNWxMibpCo.git",
  revision = "29dac88ecded94457572db3fdfaaaab95fa91525",
  license = "AGPL-3.0-or-later",
  package = "bounded-exec",
}
in
let Bounds = {
  timeout_ms = TimeoutMilliseconds,
  stdin_bytes = StreamBytes,
  stdout_bytes = StreamBytes,
  stderr_bytes = StreamBytes,
  poll_interval_ms = PollIntervalMilliseconds,
  teardown_timeout_ms = TeardownTimeoutMilliseconds,
  arguments = ArgumentCount,
  argument_bytes = ArgumentBytes,
  environment_entries = EnvironmentEntryCount,
  environment_name_bytes = EnvironmentNameBytes,
  environment_value_bytes = EnvironmentValueBytes,
  concurrency_units = ConcurrencyUnits,
  queue_units = QueueUnits,
}
in
let Common = {
  schema = "molten.fabric.execution-profile.v1",
  environment_mode = "clear",
  invocation_mode = "direct",
  executable_resolution = "exact-artifact",
  workspace_mode = "capability-root",
  component = Component,
  bounds = Bounds,
  termination_scopes = ["process-group"],
  non_claims = Contracts.RequiredNonClaims,
}
in
{
  live | Contracts.ExecutionProfile =
    Common
    & {
      profile_id = "bounded-process-live-v1",
      kind = "live-bounded-process",
      platform = "unix-process-group",
    },
  simulation | Contracts.ExecutionProfile =
    Common
    & {
      profile_id = "bounded-process-simulation-v1",
      kind = "deterministic-simulation",
      platform = "unix-process-group",
    },
}
