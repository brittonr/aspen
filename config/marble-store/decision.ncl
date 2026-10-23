let Contracts = import "contracts.ncl" in
let Small = "prolly-64-entries-1-update" in
let Large = "prolly-4096-entries-32x16-updates" in
let row = fun workload backend_name metric_name observed =>
  {
    workload_ref = workload,
    backend = backend_name,
    metric = metric_name,
    unit = if metric_name == "space-amplification" then "permille" else "nanoseconds",
    value = observed,
  }
in
{
  schema = "molten.marble-store-decision.v1",
  verdict = "keep-current",
  claim_verb = "records",
  workloads = [
    {
      workload_ref = Small,
      description = "One 64-entry Prolly map publication and one single-key update, then GC of unreachable blocks; mean per publication and per block read.",
    },
    {
      workload_ref = Large,
      description = "One 4096-entry Prolly map publication and 32 update rounds of 16 keys each, then GC of unreachable blocks; mean per publication and per block read.",
    },
  ],
  host = "Linux 6.18.50 x86_64, 32 threads, ZFS dataset datapool/tmp, unoptimized dev profile, marble 16.0.2 with fsync_each_batch, art 1.0.0, read cache 64 entries, maintenance every 4 batches, one run on 2026-10-22",
  measurements = [
    row Small "redb-current" "batch-write" 2009037,
    row Small "redb-current" "point-read" 7018,
    row Small "redb-current" "recovery" 1650719,
    row Small "redb-current" "space-amplification" 91142,
    row Small "marble-art" "batch-write" 875789,
    row Small "marble-art" "point-read" 12458,
    row Small "marble-art" "recovery" 1663283,
    row Small "marble-art" "space-amplification" 1500,
    row Large "redb-current" "batch-write" 6953607,
    row Large "redb-current" "point-read" 6546,
    row Large "redb-current" "recovery" 39806659,
    row Large "redb-current" "space-amplification" 10698,
    row Large "marble-art" "batch-write" 6327593,
    row Large "marble-art" "point-read" 28096,
    row Large "marble-art" "recovery" 61822765,
    row Large "marble-art" "space-amplification" 1114,
  ],
  findings = [
    "point reads record 1.8x to 4.3x slower on marble-art: every uncached read crosses the executor and reads its object from the heap",
    "recovery records 1.55x slower on the large workload: art is in memory only, so every open reads every live object to rebuild the digest index",
    "batch writes record 0.44x to 0.91x of the Redb time",
    "space amplification records 1.1x to 1.5x on marble-art against 10.7x to 91x on Redb, whose file length includes preallocated pages",
    "marble 16.0.2 builds slices over uninitialized memory in its read path (uninit_boxed_slice, read_range_at), which is undefined behavior",
    "marble 16.0.2 sizes a recovery read from an on-disk length before checking a checksum and panics through assert and unwrap on inconsistent state",
    "a failed marble write_batch can leave new identifiers installed in the runtime location table, so the adapter must poison the instance until reopen",
    "marble opens its heap by ambient filesystem path, outside the capability-rooted node-state namespaces",
  ],
  non_claims = Contracts.required_non_claims,
} | Contracts.Decision
