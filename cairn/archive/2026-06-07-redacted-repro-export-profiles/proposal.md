# Change: redacted-repro-export-profiles

## Why

`sealed-repro-redaction-preflight` makes portable pass artifacts fail closed when sensitive marker records appear. That is safe, but it is not enough for real operators: legitimate repros often need to include enough structure to debug an issue while withholding payload bytes, credentials, private subjects, or encrypted references.

## What

- Add explicit repro export profiles: `deny-sensitive`, `redacted-diagnostic`, and `encrypted-private`.
- Represent redaction transforms as canonical Preserves receipts bound to the source report, output bundle, redaction policy, and profile.
- Keep default sealed pass evidence on `deny-sensitive`; redacted diagnostic bundles must not satisfy pass gates unless their transform receipt is explicitly accepted by policy.
- Add validated `<encrypted-ref ...>` handling only when encryption metadata, recipient policy, and reveal receipts are present.
- Preserve deterministic replay by recording whether redaction is lossless-for-gate, diagnostic-only, or requires authorized reveal.

## Impact

This turns the current conservative marker denial into an explicit lifecycle for safe diagnostics. It does not weaken pass gates: unreviewed redaction, malformed encrypted refs, and missing reveal receipts remain fail-closed.
