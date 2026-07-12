let Contracts = import "contracts.ncl" in
let get = fun field fallback overrides => std.record.get_or field fallback overrides in
let make_sampling = fun process_count iteration_count sample_limit => {
  processes = process_count,
  iterations_per_process = iteration_count,
  min_samples_per_phase = process_count * iteration_count,
  max_samples_per_phase = sample_limit,
} in
let suite = fun lane_name suite_name bundle_refs workload_id_refs process_count iteration_count sample_limit args => {
  lane = lane_name,
  suite_id = suite_name,
  measurement = get "measurement" "cycles" args,
  pin_to_single_core = get "pin_to_single_core" true args,
  materialization_bundle_refs = bundle_refs,
  workload_refs = workload_id_refs,
  host_class_ref = Contracts.refs.host_class,
  resource_envelope_ref = Contracts.refs.resource_envelope,
  engine_cohort_ref = Contracts.refs.engine_cohort,
  engine_artifact_ref = Contracts.refs.engine_artifact,
  runner_artifact_ref = Contracts.refs.runner_artifact,
  phases = Contracts.phases,
  sampling = make_sampling process_count iteration_count sample_limit,
} in
{
  make = fun args =>
    {
      schema_id = get "schema_id" Contracts.schema_id args,
      schema_version = get "schema_version" Contracts.schema_version args,
      source_language = get "source_language" Contracts.source_language args,
      profile = {
        profile_id = get "profile_id" Contracts.profile_id args,
        evidence_role = get "evidence_role" Contracts.evidence_role args,
        component_profile_id = get "component_profile_id" Contracts.component_profile_id args,
        sightglass = {
          revision = get "sightglass_revision" Contracts.sightglass.revision args,
          runner = get "sightglass_runner" Contracts.sightglass.runner args,
          raw_schema = get "sightglass_raw_schema" Contracts.sightglass.raw_schema args,
        },
        phases = Contracts.phases,
        fast = suite
          "fast"
          "molten-wasm-component-fast-v1"
          Contracts.refs.fast_bundles
          [Contracts.refs.actor_workload]
          Contracts.sampling.fast.processes
          Contracts.sampling.fast.iterations_per_process
          Contracts.sampling.fast.max_samples_per_phase
          args,
        deep = suite
          "deep"
          "molten-wasm-component-deep-v1"
          Contracts.refs.deep_bundles
          [Contracts.refs.actor_workload, Contracts.refs.system_extension_workload]
          Contracts.sampling.deep.processes
          Contracts.sampling.deep.iterations_per_process
          Contracts.sampling.deep.max_samples_per_phase
          args,
        comparison = Contracts.comparison,
        optimization_limits = Contracts.optimization_limits,
        non_claims = get "non_claims" Contracts.non_claims args,
      },
    } | Contracts.Export,
}
