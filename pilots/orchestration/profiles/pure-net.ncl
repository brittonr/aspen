# Reviewed local operator profile, exported with:
# nickel export --format json profiles/pure-net.ncl > limits.json
# Rust Limits::default is the mechanism ceiling, not a production qualification.
# r[impl molten.pure_net.profile]
# r[impl molten.pure_net.bounds]
{
  max_program_bytes | Number = 131072,
  max_input_bytes | Number = 32768,
  max_source_bytes | Number = 8192,
  max_source_depth | Number = 32,
  max_source_nodes | Number = 2048,
  max_frontend_work | Number = 100000,
  max_extraction_work | Number = 100000,
  max_ir_nodes | Number = 1024,
  max_ir_bytes | Number = 65536,
  max_inputs | Number = 256,
  max_outputs | Number = 256,
  max_agent_slots | Number = 8192,
  max_wires | Number = 16384,
  max_queue_entries | Number = 8192,
  max_memory_bytes | Number = 67108864,
  max_plan_scratch_bytes | Number = 16384,
  max_setup_work | Number = 2000000,
  max_validation_work | Number = 128000000,
  interaction_fuel | Number = 100000,
  reference_fuel | Number = 100000,
  max_readback_work | Number = 16384,
  max_observations | Number = 1024,
  quantum_work | Number = 256,
  max_atomic_work | Number = 16000000,
  max_total_work | Number = 256000000,
  max_attempts | Number = 2,
  max_aggregate_attempt_work | Number = 512000000,
  max_concurrent_evaluations | Number = 1,
  process_address_space_bytes | Number = 8589934592,
  process_cpu_soft_seconds | Number = 120,
  process_cpu_hard_seconds | Number = 121,
}
