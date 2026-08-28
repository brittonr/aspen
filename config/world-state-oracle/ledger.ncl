let Contracts = import "contracts.ncl" in
let SqliteContract = "blake3:82d470f924e39e4e6eed5ce48095bcb30e682b15e076476a1caf847dac9ab664" in
let ConcurrencyContract = "blake3:9efcaf8c67d3b1d6c1e9eac578810bfd266bb2fb920344230299db143d6afcc8" in
let FormatContract = "blake3:9a24814b1023720459092e2fc0126c09ad6af221b6ec876948d0c67c2bcb5452" in
let row = fun row_id row_contract row_status row_evidence row_fixture row_issue row_explanation =>
  {
    id = row_id,
    source_contract = row_contract,
    status = row_status,
    evidence_ref = row_evidence,
    fixture = row_fixture,
    issue = row_issue,
    explanation = row_explanation,
  }
in
{
  schema = "molten.semantic-state-oracle-compatibility.v1",
  repository = "https://github.com/dolthub/doltlite",
  source_revision = "10170ed82c1b12414db8d1b29d2fe9ea2a72fd88",
  backend_format = "doltlite-chunk-store-v12",
  remotes_enabled = false,
  vec1_enabled = false,
  claims_molten_correctness = false,
  claims_global_root_identity = false,
  production_enabled = false,
  limits = {
    max_adapted = 2,
    max_intentional = 7,
    max_unsupported = 1,
    max_engine_gap = 0,
  },
  rows = [
    row "authority" "molten.authority" "intentional" SqliteContract "authority-nonclaim" null "Molten retains authority admission",
    row "branch-isolation" "concurrency.per-connection-branch" "compatible" ConcurrencyContract "branch-isolation" null "branch-visible state stays isolated",
    row "compare-and-advance" "concurrency.vc-head-recheck" "compatible" ConcurrencyContract "compare-and-advance" null "stale tips do not replace a winner",
    row "complete-world-atomicity" "molten.complete-world" "intentional" SqliteContract "multi-file-negative" null "Molten commits a complete world under its own protocol",
    row "custom-collation" "sqlite.persisted-custom-collation" "adapted" SqliteContract "custom-collation-negative" null "oracle schemas use built-in binary ordering",
    row "detached-read" "branch.detached-revision" "compatible" SqliteContract "detached-read" null "detached snapshots remain read-only",
    row "durable-conflicts" "molten.durable-conflicts" "intentional" ConcurrencyContract "conflict-nonclaim" null "Molten persists typed conflict artifacts",
    row "effect-release" "molten.effect-release" "intentional" SqliteContract "effect-nonclaim" null "Molten owns effect reservation and dispatch",
    row "exact-format" "storage.exact-version" "compatible" FormatContract "exact-format" null "format version twelve reopens and other versions deny",
    row "explicit-primary-key" "sqlite.clustered-primary-key" "adapted" SqliteContract "rowid-negative" null "oracle rows use explicit canonical primary keys",
    row "global-identities" "molten.blake3-identities" "intentional" FormatContract "identity-overclaim" null "backend hashes stay local evidence",
    row "history-independent-state" "storage.history-independent-table-root" "compatible" FormatContract "history-independent" null "equal primary-key state has equal backend roots",
    row "multi-file-write" "sqlite.multi-file-atomic-write" "unsupported" SqliteContract "multi-file-write-negative" "dolthub/doltlite#storage-multi-file" "DoltLite rejects multiple file-backed writes",
    row "reader-safe-gc" "concurrency.gc-reader" "compatible" ConcurrencyContract "reader-safe-gc" null "open readers retain committed observations during GC",
    row "retention-policy" "molten.retention" "intentional" ConcurrencyContract "retention-nonclaim" null "Molten owns retention and deletion admission",
    row "serialization" "sqlite.serialize-native-image" "compatible" SqliteContract "serialization" null "native images round trip inside the pinned cohort",
    row "stale-writer" "concurrency.snapshot-upgrade" "compatible" ConcurrencyContract "stale-writer" null "stale read snapshots deny write upgrade",
    row "typed-merge-policy" "molten.typed-merge" "intentional" SqliteContract "merge-nonclaim" null "Molten owns typed merge decisions",
  ],
} | Contracts.Profile
