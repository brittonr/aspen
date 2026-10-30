let P = import "../../docs/nickel-contract-prelude.ncl" in
let Ref = P.contract P.is_blake3_ref in
let PackRef = P.contract (fun v => std.is_string v && std.string.is_match "^onix:blake3:bpf-pack:[0-9a-f]{64}$" v) in
let KernelRef = P.contract (fun v => std.is_string v && std.string.is_match "^onix:blake3:kernel-build:[0-9a-f]{64}$" v) in
let Token = P.contract (fun v => P.is_stable_id v && std.string.length v <= 256) in
let Bound = fun maximum => P.contract (fun v => P.is_positive_integer v && v <= maximum) in
let Exact = fun expected => P.contract (P.is_exact_string expected) in
let NonClaims = [
  "observation-does-not-grant-capability-authority",
  "finding-does-not-grant-repair-authority",
  "health-does-not-prove-service-correctness",
  "local-observation-does-not-prove-cluster-truth",
  "readiness-does-not-prove-release-eligibility",
  "redaction-does-not-prove-confidentiality",
  "scan-does-not-prove-completeness-beyond-plan",
] in
let Binding = {
  machine | Token,
  generation | Token,
  pack | PackRef,
  program | Ref,
  object | Ref,
  kernel | KernelRef,
  btf | Ref,
  schema | Exact "onixos.bpf-observation.window.v1",
  scope | Ref,
  accounting | Ref,
  adapter | Ref,
} in
let Mapping = {
  signal | P.contract (P.is_one_of ["process", "scheduler", "block", "network", "pressure", "adapter-health"]),
  descriptor_ref | Ref,
  aggregation | Exact "sum",
  labels | P.contract (fun v => std.is_array v && std.array.length v == 0),
} in
let Profile = {
  schema | Exact "molten.linux-ebpf-observation.profile.v1",
  profile_ref | Ref,
  binding | Binding,
  mappings | Array Mapping
    | P.contract (fun v => P.is_non_empty_bounded_array 6 v
      && P.distinct_by (fun m => m.signal) v && P.distinct_by (fun m => m.descriptor_ref) v),
  max_records | Bound 65536,
  max_bytes | Bound 4194304,
  max_streams | Bound 4096,
  max_queue_records | Bound 65536,
  max_queue_bytes | Bound 4194304,
  max_series | Bound 6,
  max_events | P.contract (P.is_exact_integer 1),
  window_ticks | Bound 1000000000000,
  freshness_ticks | Bound 1000000000000,
  required | Bool,
  loss_policy | Exact "incomplete-on-any-loss-or-missing-accounting",
  event_detail | Exact "status-only",
  redaction | Exact "deny-unmapped-and-all-free-text",
  scope | Exact "local-component",
  non_claims | P.contract (fun v => v == NonClaims),
} in
{ Profile = Profile, Binding = Binding, Mapping = Mapping, non_claims = NonClaims }
