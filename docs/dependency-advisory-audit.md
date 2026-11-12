# Resolved dependency advisory and license audit

`tools/dependency-audit.rs` is a repeatable, read-only report over the resolved
`cargo metadata` closure. It records the exact metadata and policy refs, the
advisory snapshot revision and ref, source and license findings, dependency
paths, dispositions, and a compact Markdown summary.

It does not replace the owned-package license checker, the release dependency
profile, legal review, or runtime security qualification. A clean report is not
a vulnerability-free claim.

## Inputs

The policy is authored in Nickel and exported deterministically:

```sh
nickel export --format json config/dependency-audit/policy.ncl > /tmp/policy.json
```

The advisory snapshot is a reviewed external observation:

```json
{
  "schema": "molten.advisory-snapshot.v1",
  "revision": "advisory-db-2026-11-12",
  "generated_at": "2026-11-12",
  "advisories": [
    {
      "id": "RUSTSEC-0000-0000",
      "package": "example",
      "versions": ["1.2.3"],
      "severity": "high"
    }
  ]
}
```

`versions` accepts exact versions or `"*"`. The tool does not fetch or infer
advisory data; a missing snapshot is reported as unavailable and denies the
report rather than becoming a clean result.

## Reviewed exceptions

Exceptions live in the policy and bind package, source, advisory id (or `null`
for a package-wide review), owner, rationale, and ISO expiry date:

```json
{
  "package": "example",
  "source": "registry+https://github.com/rust-lang/crates.io-index",
  "advisory": "RUSTSEC-0000-0000",
  "owner": "dependency-owner",
  "rationale": "bounded exposure review",
  "expires": "2026-12-31"
}
```

An exception whose `expires` is earlier than `--as-of` becomes actionable. An
exception is not a clean result; it is a recorded disposition with an owner and
expiry.

## Run

```sh
cargo metadata --locked --format-version 1 > /tmp/metadata.json
cargo -q -Zscript tools/dependency-audit.rs \
  --metadata /tmp/metadata.json \
  --policy /tmp/policy.json \
  --advisories /tmp/advisories.json \
  --as-of 2026-11-12 \
  --markdown-out dependency-audit.md \
  --json-out dependency-audit.json
```

The wrapper does the Nickel export and metadata capture:

```sh
scripts/dependency-audit.sh 2026-11-12 --advisories /tmp/advisories.json --markdown-out dependency-audit.md
```

The report denies when an advisory snapshot is missing, a source is outside the
policy, a license expression is unavailable or outside the allowlist, an
advisory has no current exception, or an exception is expired. `--self-test`
covers an affected-package advisory, an unrecognized source, missing license
metadata, a current exception, an expired exception, and a missing advisory
snapshot.
