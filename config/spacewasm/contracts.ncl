let P = import "../../docs/nickel-contract-prelude.ncl" in
let Exact = fun v => P.contract (P.is_exact_string v) in
let Bound = fun maximum => P.contract (fun v => P.is_positive_integer v && v <= maximum) in
let NonClaims = ["not-component-compatible", "not-portable-state", "not-cross-host-migration",
  "not-runtime-correctness", "not-sandbox-completeness", "not-production-ready", "not-release-eligible"] in
let Limits = {
  allocation_bytes | Bound 2147483647,
  ir_bytes | Bound 65536,
  stack_bytes | Bound 2147483647 | P.contract (fun v => std.number.is_integer (v / 4)),
  memory_pages | Bound 256,
  table_elements | P.contract (P.is_exact_integer 0),
  segment_fuel | Bound 9007199254740991,
  total_fuel | Bound 9007199254740991,
  hostcall_bytes | Bound 16777216,
  input_bytes | Bound 16777216,
  output_bytes | Bound 16777216,
  continuation_count | Bound 65536,
  continuation_bytes | Bound 9007199254740991,
  retention_turns | Bound 9007199254740991,
} in
let Profile = {
  name | Exact "spacewasm-core-mvp-pilot",
  source_revision | Exact "e24cf09355a90497148eb5029fdb8e3400bd63e3",
  abi | Exact "molten.wasm.abi.v1",
  limits | Limits | P.contract (fun v => v.segment_fuel <= v.total_fuel
    && v.ir_bytes <= v.allocation_bytes && v.stack_bytes <= v.allocation_bytes
    && v.hostcall_bytes <= v.memory_pages * 65536),
  production | P.contract (fun v => v == false),
  release | P.contract (fun v => v == false),
  non_claims | P.contract (fun v => v == NonClaims),
} in
{ Profile = Profile, Limits = Limits, non_claims = NonClaims }
