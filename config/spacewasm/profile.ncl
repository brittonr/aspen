# Optional authoring default: loading this file does not select an engine.
let C = import "./contracts.ncl" in
{
  name = "spacewasm-core-mvp-pilot",
  source_revision = "e24cf09355a90497148eb5029fdb8e3400bd63e3",
  abi = "molten.wasm.abi.v1",
  cohort = {
    module_features = "wasm1",
    materialization_schema = "mantle-spacewasm-reference-bundle-v1",
    materialization_profile = "nasa-spacewasm-e24cf093-reference-v1",
    octet_receipt_schema = "octet-evidence-rail-receipt/v1",
    octet_profile = "spacewasm-molten-abi-pilot",
    octet_runtime_cohort = "b3:7c537eef7bff66d0a90508715de08de79c9aaac23352e1007a6d47e7fb4674a2",
    chaos_differential_schema = "chaoscontrol.spacewasm-molten-abi-differential.v1",
    reference_engine = "wasmtime",
    reference_engine_version = "49.0.5",
  },
  limits = {
    allocation_bytes = 33554432, ir_bytes = 65536, stack_bytes = 524288,
    memory_pages = 256, table_elements = 0,
    segment_fuel = 4096, total_fuel = 1048576,
    hostcall_bytes = 65536, input_bytes = 65536, output_bytes = 65536,
    continuation_count = 16, continuation_bytes = 536870912, retention_turns = 64,
  },
  production = false, release = false, non_claims = C.non_claims,
} | C.Profile
