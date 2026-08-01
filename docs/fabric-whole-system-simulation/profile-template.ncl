let Contracts = import "contracts.ncl" in
let get = fun field fallback args => std.record.get_or field fallback args in
let MaxChoices = 256 in
let MaxEvents = 256 in
let MaxVirtualTicks = 4096 in
let MaxTraceBytes = 1048576 in
let MaxResourceUnits = 4096 in
let MaxShrinkAttempts = 256 in
{
  make = fun args => {
    schema_id = Contracts.schema_id,
    schema_version = Contracts.schema_version,
    source_language = Contracts.source_language,
    claim_profile = get "claim_profile" "deterministic-whole-system" args,
    nodes = get "nodes" [
      "node-transactional-key-value",
      "node-replicated-log",
      "node-distributed-scheduler",
    ] args,
    port_classes = get "port_classes" Contracts.port_classes args,
    workload = get "workload" [
      "transactional-key-value-commit",
      "replicated-log-append",
      "distributed-scheduler-complete",
    ] args,
    bounds = {
      max_choices = get "max_choices" MaxChoices args,
      max_events = get "max_events" MaxEvents args,
      max_virtual_ticks = get "max_virtual_ticks" MaxVirtualTicks args,
      max_trace_bytes = get "max_trace_bytes" MaxTraceBytes args,
      max_resource_units = get "max_resource_units" MaxResourceUnits args,
      max_shrink_attempts = get "max_shrink_attempts" MaxShrinkAttempts args,
    },
    ambient_inputs = get "ambient_inputs" [] args,
    non_claims = get "non_claims" Contracts.non_claims args,
  } | Contracts.Export,
}
