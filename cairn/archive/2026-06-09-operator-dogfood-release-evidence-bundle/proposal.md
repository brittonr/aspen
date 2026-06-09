## Why

Molten now preserves dogfood report, release gate, Nix dogfood evidence, and Nix verification receipts as separate release-review artifacts. Reviewers still need one canonical artifact that names the complete release evidence set and one verifier that recomputes the refs before accepting the review graph.

## What Changes

- Add canonical `release-evidence-bundle-v1` artifacts for dogfood release review.
- Add `release-evidence-bundle-verify-receipt-v1` receipts that recompute Nix dogfood output refs and deny stale, missing, or tampered bundle members.
- Add CLI commands to export and verify release evidence bundles from a realized dogfood Nix output path.
- Preserve bundle and verify receipts in the `dogfood-local-node` Nix check output.
- Keep release bundles evidence-only; they do not grant authority, policy, provenance, resource, transport, source-gate, retention, or destructive-operation trust.

## Impact

Release review can consume a single canonical bundle and verifier receipt while retaining the existing subsystem gates. CI and local operators can detect stale report/gate/Nix evidence refs without relying on logs or path names as authority.
