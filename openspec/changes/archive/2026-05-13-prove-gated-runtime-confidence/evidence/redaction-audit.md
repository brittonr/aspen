# Evidence redaction audit

Captured: 2026-05-13T05:37:54Z

## Scope

Committed evidence files scanned:

- `evidence/baseline.md`
- `evidence/cheap-runtime-host.md`
- `evidence/vm-microvm-tier.md`
- `evidence/vm-runtime-host-phase3.md`
- `evidence/hermit-uhyve-phase3.md`
- `evidence/hyperlight-phase3.md`
- `evidence/dogfood-full.md`
- `evidence/full-flake-check.md`

Raw logs remain under ignored `target/runtime-proof/` and are intentionally not committed.

## Check

A local pattern scan covered committed evidence for:

- private-key PEM markers
- credential-oriented words such as password, secret, token, cookie, credential, private key, connection string
- long hexadecimal blobs
- AWS access-key prefixes
- URI userinfo credentials

The only matches were documentation/redaction statements:

- `evidence/cheap-runtime-host.md`: `## Secret handling`
- `evidence/vm-runtime-host-phase3.md`: statement that committed summary omits cluster tickets and secret material

No committed evidence file contained private-key markers, long hex blobs, URI userinfo credentials, or provider key prefixes from this scan.

## Result

Committed evidence satisfies the change's redacted-evidence boundary: command summaries, receipt paths, exit statuses, and classifications are committed; raw runtime logs and verbose VM traces are omitted.
