let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let NoRestartMismatch = 0 in
let ExpectedAdversarialNodeBytes = 4096 in
let LocalConsumerCount = 1 in
let RequiredConsumerCount = 2 in
let Blake3Ref =
  Prelude.contract (fun value =>
    std.is_string value && std.string.is_match "^blake3:[0-9a-f]{64}$" value
  )
in
let BenchmarkShape = {
  schema | Prelude.contract (Prelude.is_exact_string "molten.prolly-map-benchmark.v1"),
  profile_ref | Blake3Ref,
  profile_name | Prelude.contracts.NonEmptyString,
  entry_count | Number,
  mutation_count | Number,
  sharing_min_reused_blocks | Number,
  diff_max_records | Number,
  retained_bytes_maximum | Number,
  gc_candidates_maximum | Number,
  restart_mismatch_maximum | Number,
  adversarial_node_bytes_maximum | Number,
  observed_logical_bytes | Number,
  observed_block_count | Number,
  observed_block_bytes | Number,
  observed_reused_blocks | Number,
  observed_diff_records | Number,
  observed_skipped_equal_nodes | Number,
  observed_gc_candidates | Number,
  observed_restart_verified | Bool,
  credible_consumers | Number,
  extraction_required_consumers | Number,
  extraction_disposition | Prelude.contracts.NonEmptyString,
  raw_timings_are_correctness | Bool,
  creates_repository | Bool,
  approves_dependency | Bool,
}
in
let invariants = fun benchmark =>
  benchmark.entry_count > 0
  && benchmark.mutation_count > 0
  && benchmark.sharing_min_reused_blocks > 0
  && benchmark.diff_max_records > 0
  && benchmark.retained_bytes_maximum > 0
  && benchmark.gc_candidates_maximum > 0
  && benchmark.restart_mismatch_maximum == NoRestartMismatch
  && benchmark.adversarial_node_bytes_maximum == ExpectedAdversarialNodeBytes
  && benchmark.observed_logical_bytes > 0
  && benchmark.observed_block_count > 0
  && benchmark.observed_block_bytes <= benchmark.retained_bytes_maximum
  && benchmark.observed_reused_blocks >= benchmark.sharing_min_reused_blocks
  && benchmark.observed_diff_records <= benchmark.diff_max_records
  && benchmark.observed_skipped_equal_nodes > 0
  && benchmark.observed_gc_candidates <= benchmark.gc_candidates_maximum
  && benchmark.observed_restart_verified == true
  && benchmark.credible_consumers == LocalConsumerCount
  && benchmark.extraction_required_consumers == RequiredConsumerCount
  && benchmark.extraction_disposition == "retain-current"
  && benchmark.raw_timings_are_correctness == false
  && benchmark.creates_repository == false
  && benchmark.approves_dependency == false
in
{
  Benchmark = std.contract.Sequence [BenchmarkShape, Prelude.contract invariants],
}
