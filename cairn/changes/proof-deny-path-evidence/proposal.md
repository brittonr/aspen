## Why

Proof strength depends as much on rejected cases as passing cases. Current gates already fail closed in many areas, but release review is easier when every gate declares the deny-path corpus it requires and preserves canonical evidence for those denials.

## What Changes

- Define a deny-path evidence matrix for proof-bearing gates.
- Require canonical denial receipts for missing artifacts, stale refs, malformed schemas, wrong signer or purpose, tampered bytes, duplicate receipts, denied mutation attempts, and diagnostic-only evidence.
- Add no-mutation evidence when denials happen before side effects.
- Use Hegel RS properties to generate malformed and stale combinations and prove they cannot pass.

## Impact

- **Files**: evidence gate validation, harness fixtures, receipt summaries, release docs.
- **Testing**: deny-path fixtures for each required class and Hegel RS generated tamper/stale cases.
