## Why

Release evidence export, bundle verification, signed-member verification, promotion, signing, readback, and archive export form a release-review state machine. These receipts remain evidence-only, but proof should show that stale, tampered, unsigned, wrong-purpose, or revoked-key members cannot pass release review or bypass subsystem gates.

## What Changes

- Add requirements for release evidence workflow state proof.
- Require proof traces over bundle export, bundle verify, signed-member verify, promotion, signed promotion verify, summary/readback, archive export, and export verify.
- Require negative evidence for missing members, duplicate paths, tampered refs, wrong signer, wrong purpose, revoked key, stale replay/proof refs, and evidence-only boundary misuse.

## Impact

- **Files**: operator dogfood release workflow, signed receipt keyring, release bundle/export verification, catalog readback, and tests.
- **Testing**: complete release workflow pass, missing/tampered/duplicate member denial, wrong-purpose signature denial, revoked-key denial, and evidence-only downstream gate denial.
