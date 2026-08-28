let Contracts = import "source-contracts.ncl" in
let TimeoutMilliseconds = 10000 in
let StdinMaximumBytes = 65536 in
let StdoutMaximumBytes = 262144 in
let StderrMaximumBytes = 65536 in
let TeardownTimeoutMilliseconds = 1000 in
let MaximumRows = 256 in
let MaximumKeyBytes = 256 in
let MaximumValueBytes = 4096 in
let MaximumDiagnostics = 32 in
{
  schema = "molten.semantic-state-oracle-source.v1",
  repository = "https://github.com/dolthub/doltlite",
  source_revision = "10170ed82c1b12414db8d1b29d2fe9ea2a72fd88",
  source_input = "github:dolthub/doltlite/10170ed82c1b12414db8d1b29d2fe9ea2a72fd88",
  adapter_version = "molten-doltlite-oracle-v1",
  backend_format = "doltlite-chunk-store-v12",
  binary_ref = "blake3:019983d04bbbd689aec0faac418f99fa49f2f61e888bed8d9e0f34cfc8b3e08b",
  adapter_ref = "blake3:a4b36a1a46cff61a2c4efb5fa4bbd26778467d65b9cc511197470607ac3e393a",
  imported_scope = [
    "flake-input:full-pinned-source-checkout",
    "build:non-amalgamation-cli-and-library",
    "src/doltlite_*.c",
    "src/prolly_*.c",
    "src/chunk_*.c",
    "test/sqlite_compatibility_contract.tsv",
    "test/concurrency_contract.tsv",
    "test/storage_format_contract.tsv",
  ],
  build_inputs = ["stdenv.cc", "gnumake", "zlib"],
  notices = [
    {
      path = "APACHE_LICENSE",
      license = "Apache-2.0",
      content_ref = "blake3:a24e4e2958e399474e4b0913dde32c6be84630b6dcf153af7eae29779399eb2f",
    },
    {
      path = "LICENSE.md",
      license = "SQLite-public-domain-and-project-notices",
      content_ref = "blake3:4f91d1a7d7b99eefb5c81ddb148446616d8260fc0c8113999cf2a48d3589267c",
    },
  ],
  contract_artifacts = [
    {
      path = "test/sqlite_compatibility_contract.tsv",
      content_ref = "blake3:82d470f924e39e4e6eed5ce48095bcb30e682b15e076476a1caf847dac9ab664",
    },
    {
      path = "test/concurrency_contract.tsv",
      content_ref = "blake3:9efcaf8c67d3b1d6c1e9eac578810bfd266bb2fb920344230299db143d6afcc8",
    },
    {
      path = "test/storage_format_contract.tsv",
      content_ref = "blake3:9a24814b1023720459092e2fc0126c09ad6af221b6ec876948d0c67c2bcb5452",
    },
  ],
  remotes_enabled = false,
  vec1_enabled = false,
  environment_cleared = true,
  absolute_paths_required = true,
  capability_rooted_workspace = true,
  process_group_teardown = true,
  test_only = true,
  production_enabled = false,
  authority_granted = false,
  claims_molten_correctness = false,
  claims_global_root_identity = false,
  bounds = {
    timeout_ms = TimeoutMilliseconds,
    stdin_max_bytes = StdinMaximumBytes,
    stdout_max_bytes = StdoutMaximumBytes,
    stderr_max_bytes = StderrMaximumBytes,
    teardown_timeout_ms = TeardownTimeoutMilliseconds,
    max_rows = MaximumRows,
    max_key_bytes = MaximumKeyBytes,
    max_value_bytes = MaximumValueBytes,
    max_diagnostics = MaximumDiagnostics,
  },
  non_claims = [
    "oracle-agreement-is-not-correctness",
    "backend-roots-are-not-global-identities",
    "oracle-evidence-grants-no-authority",
    "oracle-evidence-is-not-production-readiness",
  ],
} | Contracts.Source
