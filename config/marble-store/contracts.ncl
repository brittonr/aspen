let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let MaximumQueueDepth = 1024 in
let MaximumReadCacheEntries = 65536 in
let MaximumObjectBytes = 16777216 in
let FrameHeaderBytes = 33 in
let Metrics = ["batch-write", "point-read", "recovery", "space-amplification"] in
let Backends = ["redb-current", "marble-art"] in
let RequiredNonClaims = [
  "the spike changes no dataspace, vat, networking, or replication semantics",
  "the spike does not change the default storage path",
  "marble object identifiers are physical handles, not content identity",
  "measurements record one workload on one host and do not prove runtime superiority",
  "durability covers only batches marble atomically recovered",
]
in
let IdRange = {
  start | Prelude.contract Prelude.is_non_negative_integer,
  end | Prelude.contract Prelude.is_positive_integer,
}
in
let SelectionShape = {
  schema | Prelude.contract (Prelude.is_exact_string "molten.prolly-store-selection.v1"),
  backend | Prelude.contract (Prelude.is_one_of ["redb", "marble-art"]),
  marble
    | {
      catalog_ids | IdRange,
      block_ids | IdRange,
      maintenance_every_committed_batches | Prelude.contract Prelude.is_positive_integer,
      executor_queue_depth | Prelude.contract Prelude.is_positive_integer,
      read_cache_entries | Prelude.contract Prelude.is_non_negative_integer,
      max_object_bytes | Prelude.contract Prelude.is_positive_integer,
    },
  non_claims | Prelude.contracts.NonEmptyUniqueStringArray,
}
in
let selection_invariants = fun selection =>
  let marble = selection.marble in
  marble.catalog_ids.start < marble.catalog_ids.end
  && marble.block_ids.start < marble.block_ids.end
  && (
    marble.catalog_ids.end <= marble.block_ids.start
    || marble.block_ids.end <= marble.catalog_ids.start
  )
  && marble.executor_queue_depth <= MaximumQueueDepth
  && marble.read_cache_entries <= MaximumReadCacheEntries
  && marble.max_object_bytes > FrameHeaderBytes
  && marble.max_object_bytes <= MaximumObjectBytes
  && Prelude.has_all selection.non_claims RequiredNonClaims
in
let Measurement = {
  workload_ref | Prelude.contracts.NonEmptyString,
  backend | Prelude.contract (Prelude.is_one_of Backends),
  metric | Prelude.contract (Prelude.is_one_of Metrics),
  unit | Prelude.contract (Prelude.is_one_of ["nanoseconds", "permille"]),
  value | Prelude.contract Prelude.is_non_negative_integer,
}
in
let DecisionShape = {
  schema | Prelude.contract (Prelude.is_exact_string "molten.marble-store-decision.v1"),
  verdict | Prelude.contract (Prelude.is_one_of ["keep-current", "replace", "inconclusive"]),
  claim_verb | Prelude.contract (Prelude.is_exact_string "records"),
  workloads | Array { workload_ref | Prelude.contracts.NonEmptyString, description | Prelude.contracts.NonEmptyString },
  host | Prelude.contracts.NonEmptyString,
  measurements | Array Measurement,
  findings | Prelude.contracts.NonEmptyUniqueStringArray,
  non_claims | Prelude.contracts.NonEmptyUniqueStringArray,
}
in
let unit_of = fun metric => if metric == "space-amplification" then "permille" else "nanoseconds" in
let measured = fun decision workload backend metric =>
  std.array.any
    (fun item => item.workload_ref == workload && item.backend == backend && item.metric == metric)
    decision.measurements
in
let value_of = fun decision workload backend metric =>
  (
    std.array.first (
      std.array.filter
        (fun item => item.workload_ref == workload && item.backend == backend && item.metric == metric)
        decision.measurements
    )
  ).value
in
let is_complete = fun decision =>
  std.array.length decision.measurements > 0
  && std.array.all
    (fun item =>
      std.array.all
        (fun backend => std.array.all (fun metric => measured decision item.workload_ref backend metric) Metrics)
        Backends
    )
    decision.measurements
in
let is_replace_supported = fun decision =>
  std.array.all
    (fun item =>
      item.backend != "marble-art"
      || item.value <= value_of decision item.workload_ref "redb-current" item.metric
    )
    decision.measurements
in
let decision_invariants = fun decision =>
  std.array.all (fun item => item.unit == unit_of item.metric) decision.measurements
  && Prelude.distinct_by
    (fun item => "%{item.workload_ref}/%{item.backend}/%{item.metric}")
    decision.measurements
  && std.array.all
    (fun item => std.array.elem item.workload_ref (std.array.map (fun workload => workload.workload_ref) decision.workloads))
    decision.measurements
  && (decision.verdict == "inconclusive" || is_complete decision)
  && (decision.verdict != "replace" || is_replace_supported decision)
  && Prelude.has_all decision.non_claims RequiredNonClaims
in
{
  Selection = std.contract.Sequence [SelectionShape, Prelude.contract selection_invariants],
  ReviewedSelection =
    std.contract.Sequence [
      SelectionShape,
      Prelude.contract selection_invariants,
      Prelude.contract (fun selection => selection.backend == "redb")
    ],
  Decision = std.contract.Sequence [DecisionShape, Prelude.contract decision_invariants],
  required_non_claims = RequiredNonClaims,
}
