let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let MaximumDependencyRows = 64 in
let MaximumArchiveReceipts = 16 in
let MaximumEvidenceFiles = 16 in
let MaximumDistributionArtifacts = 64 in
let ExactRevision = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^[0-9a-f]{40}$" value
) in
let Blake3Hex = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^[0-9a-f]{64}$" value
) in
let HttpsSource = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^https://[A-Za-z0-9._/-]+$" value
) in
let GitSource = Prelude.contract (fun value =>
  std.is_string value && std.string.is_match "^(https://|ssh://git@)github.com/[A-Za-z0-9._/-]+$" value
) in
let SafeRelativePath = Prelude.contract (fun value =>
  std.is_string value
  && std.string.is_match "^[A-Za-z0-9][A-Za-z0-9._/-]*$" value
  && !std.string.is_match "(^|/)\\.\\.(/|$)" value
  && !std.string.is_match "(^|/)\\.(/|$)" value
) in
let SourceKind = Prelude.contract (Prelude.is_one_of ["git"]) in
let TransportPolicy = Prelude.contract (Prelude.is_one_of ["https", "ssh-pinned-nix-archive"]) in
let ReleaseDisposition = Prelude.contract (Prelude.is_one_of ["runtime", "development"]) in
let ArchiveStatus = Prelude.contract (Prelude.is_exact_string "archived") in
let DependencyContract = {
  manifest_dependency | Prelude.contracts.StableId,
  package_name | Prelude.contracts.StableId,
  package_version | Prelude.contracts.Semver,
  source_kind | SourceKind,
  source_coordinate | GitSource,
  immutable_revision | ExactRevision,
  nix_input | Prelude.contracts.StableId,
  transport_policy | TransportPolicy,
  disposition | ReleaseDisposition,
} in
let EvidenceFile = {
  relative_path | SafeRelativePath,
  blake3 | Blake3Hex,
} in
let ArchiveReceiptContract = {
  id | Prelude.contracts.StableId,
  source_coordinate | GitSource,
  source_revision | ExactRevision,
  nix_input | Prelude.contracts.StableId,
  archive_path | SafeRelativePath,
  status | ArchiveStatus,
  evidence_files | std.contract.Sequence [
    Prelude.contract (Prelude.is_non_empty_bounded_array MaximumEvidenceFiles),
    Array EvidenceFile,
  ],
  non_claims | Prelude.contracts.NonEmptyUniqueStringArray,
} in
let CanonicalValence = {
  source_coordinate | GitSource,
  immutable_revision | ExactRevision,
} in
let DistributionContract = {
  license | Prelude.contract (Prelude.is_exact_string "AGPL-3.0-or-later"),
  source_coordinate | HttpsSource,
  source_revision | ExactRevision,
  notice_artifacts | std.contract.Sequence [
    Prelude.contract (Prelude.is_non_empty_bounded_array MaximumDistributionArtifacts),
    Array SafeRelativePath,
    Prelude.contracts.UniqueStringArray,
  ],
  source_export_artifacts | std.contract.Sequence [
    Prelude.contract (Prelude.is_non_empty_bounded_array MaximumDistributionArtifacts),
    Array SafeRelativePath,
    Prelude.contracts.UniqueStringArray,
  ],
  non_claims | Prelude.contracts.NonEmptyUniqueStringArray,
} in
let profile_invariants = fun profile =>
  Prelude.non_empty_distinct_by (fun row => row.manifest_dependency) profile.dependencies
  && Prelude.non_empty_distinct_by (fun row => row.nix_input) profile.dependencies
  && Prelude.non_empty_distinct_by (fun receipt => receipt.id) profile.archive_receipts
  && std.array.length profile.dependencies <= MaximumDependencyRows
  && std.array.length profile.archive_receipts <= MaximumArchiveReceipts in
let ProfileContract = std.contract.Sequence [{
  schema | Prelude.contract (Prelude.is_exact_string "molten.release-dependency-profile.v1"),
  dependencies | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array DependencyContract],
  canonical_valence | CanonicalValence,
  archive_receipts | std.contract.Sequence [Prelude.contracts.NonEmptyArray, Array ArchiveReceiptContract],
  distribution | DistributionContract,
}, Prelude.contract profile_invariants] in
{
  Profile = ProfileContract,
  Dependency = DependencyContract,
  ArchiveReceipt = ArchiveReceiptContract,
  Distribution = DistributionContract,
}
