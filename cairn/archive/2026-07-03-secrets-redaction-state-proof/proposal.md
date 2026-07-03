## Why

Secrets, encrypted refs, reveal/decrypt receipts, redaction transforms, private bundle profiles, and cleanup receipts form confidentiality state machines. A malformed profile or stale reveal receipt must not expose plaintext or convert diagnostic redaction into gate-preserving evidence.

## What Changes

- Add requirements for reveal/decrypt/redaction/cleanup lifecycle proof traces.
- Require proof that public and diagnostic profiles remain non-revealing by default.
- Require negative evidence for missing authority, stale reveal receipts, mismatched encrypted-ref ids, non-gate-preserving transforms, and cleanup without retention admission.

## Impact

- **Files**: secrets module, repro redaction/export paths, reveal/decrypt gates, cleanup receipts, and confidentiality tests.
- **Testing**: authorized reveal pass, ciphertext-only denial, stale/mismatched reveal denial, diagnostic redaction denial as pass evidence, and no plaintext default rendering.
