# Positive and negative unified-source migration tests.
let Migration = import "../lib/artifact-source-migration-receipt.ncl" in
let Receipt = import "../evidence/source/artifact-workspace-migration-v1.ncl" in
let Digest = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
let MissingCommit = "1111111111111111111111111111111111111111" in
let Result = fun candidate => Migration.validate candidate in
let Accepted = fun candidate => Result candidate |> match { 'Ok => true, 'Error _ => false } in
let RejectedAs = fun code candidate => Result candidate |> match { 'Ok => false, 'Error error => error.code == code } in
{
  tests =
    [
      Accepted Receipt,
      Migration.is_blake3 Digest,
      Migration.is_git_oid Receipt.source.revision,
      RejectedAs "migration-shape" {},
      RejectedAs "migration-header" (Receipt & { status | force = "pending" }),
      RejectedAs "migration-source" (Receipt & { source.revision | force = MissingCommit }),
      RejectedAs "migration-source" (Receipt & { source.packages | force = ["artifact-auth-core"] }),
      RejectedAs "migration-source" (Receipt & { source.license | force = "AGPL-3.0-or-later" }),
      RejectedAs "migration-consumer" (Receipt & { consumer.transfer_package_resolved | force = true }),
      RejectedAs "migration-consumer" (Receipt & { consumer.single_nix_input | force = false }),
      RejectedAs "migration-parity" (Receipt & { parity.core_blake3 | force = Digest }),
      RejectedAs "migration-parity" (Receipt & { parity.rust_implementation_changed | force = true }),
      RejectedAs "migration-validation" (Receipt & { validation.post_binding | force = "failed" }),
      RejectedAs "migration-history" (Receipt & { history.status | force = "active-fallback" }),
      RejectedAs "migration-rollback" (Receipt & { rollback.automatic | force = true }),
      RejectedAs "migration-non-claims" (Receipt & { non_claims | force = [] }),
      RejectedAs "migration-evidence" (Receipt & { evidence | force = ["/tmp/evidence.json"] }),
      RejectedAs "migration-overclaim" (Receipt & { claims = ["release-ready"] }),
      !Migration.is_blake3 "sha256:%{Digest}",
      !Migration.is_git_oid Digest,
    ]
    |> std.test.assert_all,
}
