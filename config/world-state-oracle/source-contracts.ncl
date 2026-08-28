let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let Blake3Ref =
  Prelude.contract (fun value =>
    std.is_string value && std.string.is_match "^blake3:[0-9a-f]{64}$" value
  )
in
let SourceRevision =
  Prelude.contract (fun value =>
    std.is_string value && std.string.is_match "^[0-9a-f]{40}$" value
  )
in
let NoticeContract = {
  path | Prelude.contracts.NonEmptyString,
  license | Prelude.contracts.NonEmptyString,
  content_ref | Blake3Ref,
}
in
let ContractArtifact = {
  path | Prelude.contracts.NonEmptyString,
  content_ref | Blake3Ref,
}
in
let BoundsContract = {
  timeout_ms | Number,
  stdin_max_bytes | Number,
  stdout_max_bytes | Number,
  stderr_max_bytes | Number,
  teardown_timeout_ms | Number,
  max_rows | Number,
  max_key_bytes | Number,
  max_value_bytes | Number,
  max_diagnostics | Number,
}
in
let ExpectedScope = [
  "flake-input:full-pinned-source-checkout",
  "build:non-amalgamation-cli-and-library",
  "src/doltlite_*.c",
  "src/prolly_*.c",
  "src/chunk_*.c",
  "test/sqlite_compatibility_contract.tsv",
  "test/concurrency_contract.tsv",
  "test/storage_format_contract.tsv",
]
in
let ExpectedBuildInputs = ["stdenv.cc", "gnumake", "zlib"] in
let ExpectedNonClaims = [
  "oracle-agreement-is-not-correctness",
  "backend-roots-are-not-global-identities",
  "oracle-evidence-grants-no-authority",
  "oracle-evidence-is-not-production-readiness",
]
in
let source_invariants = fun source =>
  source.source_revision == "10170ed82c1b12414db8d1b29d2fe9ea2a72fd88"
  && source.adapter_version == "molten-doltlite-oracle-v1"
  && source.imported_scope == ExpectedScope
  && source.build_inputs == ExpectedBuildInputs
  && source.remotes_enabled == false
  && source.vec1_enabled == false
  && source.environment_cleared == true
  && source.absolute_paths_required == true
  && source.capability_rooted_workspace == true
  && source.process_group_teardown == true
  && source.test_only == true
  && source.production_enabled == false
  && source.authority_granted == false
  && source.claims_molten_correctness == false
  && source.claims_global_root_identity == false
  && source.non_claims == ExpectedNonClaims
  && source.notices == [
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
  ]
  && source.contract_artifacts == [
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
  ]
in
let SourceContract =
  std.contract.Sequence [
    {
      schema | Prelude.contract (Prelude.is_exact_string "molten.semantic-state-oracle-source.v1"),
      repository | Prelude.contract (Prelude.is_exact_string "https://github.com/dolthub/doltlite"),
      source_revision | SourceRevision,
      source_input | Prelude.contracts.NonEmptyString,
      adapter_version | Prelude.contract (Prelude.is_exact_string "molten-doltlite-oracle-v1"),
      backend_format | Prelude.contract (Prelude.is_exact_string "doltlite-chunk-store-v12"),
      binary_ref | Blake3Ref,
      adapter_ref | Blake3Ref,
      imported_scope | Array Prelude.contracts.NonEmptyString,
      build_inputs | Array Prelude.contracts.NonEmptyString,
      notices | Array NoticeContract,
      contract_artifacts | Array ContractArtifact,
      remotes_enabled | Bool,
      vec1_enabled | Bool,
      environment_cleared | Bool,
      absolute_paths_required | Bool,
      capability_rooted_workspace | Bool,
      process_group_teardown | Bool,
      test_only | Bool,
      production_enabled | Bool,
      authority_granted | Bool,
      claims_molten_correctness | Bool,
      claims_global_root_identity | Bool,
      bounds | BoundsContract,
      non_claims | Array Prelude.contracts.NonEmptyString,
    },
    Prelude.contract source_invariants,
  ]
in
{
  Source = SourceContract,
  Bounds = BoundsContract,
  Notice = NoticeContract,
  ContractArtifact = ContractArtifact,
}
