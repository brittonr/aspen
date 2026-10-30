# Optional authoring default: loading this file does not select an engine.
let C = import "./contracts.ncl" in
{
  name = "spacewasm-core-mvp-pilot",
  source_revision = "e24cf09355a90497148eb5029fdb8e3400bd63e3",
  abi = "molten.wasm.abi.v1",
  limits = {
    allocation_bytes = 33554432, ir_bytes = 65536, stack_bytes = 524288,
    memory_pages = 256, table_elements = 0,
    segment_fuel = 4096, total_fuel = 1048576,
    hostcall_bytes = 65536, input_bytes = 65536, output_bytes = 65536,
    continuation_count = 16, continuation_bytes = 536870912, retention_turns = 64,
  },
  production = false, release = false, non_claims = C.non_claims,
} | C.Profile
