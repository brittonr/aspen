# Positive and negative Molten cutover evidence tests.
let cutover = import "../lib/artifact-auth-cutover-receipt.ncl" in
let receipt = import "../evidence/radicle/artifact-auth-cutover-v1.ncl" in
let digest_a = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
let missing_commit = "1111111111111111111111111111111111111111" in
let unknown_rid = "rad:z3gqcJUoA1n9HaHKufZs5FCSGazv5" in
let github_source = "ssh://git@github.com/OnixResearch/artifact-auth.git" in
let result = fun candidate => cutover.validate candidate in
let accepted = fun candidate => result candidate |> match { 'Ok => true, 'Error _ => false } in
let rejected_as = fun code candidate =>
  result candidate |> match { 'Ok => false, 'Error error => error.code == code }
in
{
  tests =
    [
      accepted receipt,
      cutover.is_blake3 digest_a,
      cutover.is_git_oid receipt.source.reviewed_commit,
      cutover.is_rid receipt.source.rid,
      rejected_as "cutover-shape" {},
      rejected_as "cutover-header" (receipt & { status | force = "pending" }),
      rejected_as "cutover-source" (receipt & { source.rid | force = unknown_rid }),
      rejected_as "cutover-source" (receipt & { source.https_git | force = github_source }),
      rejected_as "cutover-source" (receipt & { source.reviewed_commit | force = missing_commit }),
      rejected_as "cutover-source" (receipt & { source.source_archive_blake3 | force = digest_a }),
      rejected_as "cutover-publication" (receipt & { publication.receipt_blake3 | force = digest_a }),
      rejected_as "cutover-cargo" (receipt & { cargo.root_manifest_blake3 | force = digest_a }),
      rejected_as "cutover-cargo" (receipt & { cargo.lock_source | force = "git+%{github_source}" }),
      rejected_as "cutover-cargo" (receipt & { cargo.packages | force = ["artifact-auth-core"] }),
      rejected_as "cutover-nix" (receipt & { nix.locked_url | force = github_source }),
      rejected_as "cutover-nix" (receipt & { nix.nar_hash | force = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=" }),
      rejected_as "cutover-policy" (receipt & { release_policy.source_coordinate | force = github_source }),
      rejected_as "cutover-policy" (receipt & { release_policy.artifact_auth_rows | force = 1 }),
      rejected_as "cutover-unit2nix" (receipt & { unit2nix.default_plan_blake3 | force = digest_a }),
      rejected_as "cutover-unit2nix" (receipt & { unit2nix.release_packages | force = [] }),
      rejected_as "cutover-validation" (receipt & { validation.post_cutover_shell | force = "failed" }),
      rejected_as "cutover-validation" (receipt & { validation.rust_implementation_changed | force = true }),
      rejected_as "cutover-fallback" (receipt & { fallback.default_plan_github_source | force = true }),
      rejected_as "cutover-fallback" (receipt & { fallback.automatic_fallback | force = true }),
      rejected_as "cutover-rollback" (receipt & { rollback.automatic | force = true }),
      rejected_as "cutover-authority" (receipt & { authority_boundary.standalone_authority_admitted | force = true }),
      rejected_as "cutover-non-claims" (receipt & { non_claims | force = [] }),
      rejected_as "cutover-evidence" (receipt & { evidence | force = ["/tmp/acceptance.json"] }),
      rejected_as "cutover-overclaim" (receipt & { claims = ["release-ready"] }),
      rejected_as "cutover-overclaim" (receipt & { fallback_url = github_source }),
      !cutover.is_blake3 "sha256:%{digest_a}",
      !cutover.is_git_oid digest_a,
      !cutover.is_rid "rad:../private",
    ]
    |> std.test.assert_all,
}
