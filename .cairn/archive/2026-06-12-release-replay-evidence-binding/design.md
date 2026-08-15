# Design: Release Replay Evidence Binding

## Overview

Dogfood now generates a deterministic replay index during the local-node workflow. The release gate records the replay index ref in a `replay-indexes` field and includes explicit evidence-only checks. Nix dogfood release evidence reads `replay-evidence-index.preserves`, verifies it is a passing `deterministic-replay-index-v1`, and verifies the release gate binds its canonical ref.

## Readback

Release readback compares replay index refs across:

- `release-gate.preserves`
- `replay-evidence-index.preserves`
- `nix-dogfood-evidence.preserves`
- `nix-dogfood-verify.preserves`
- `release-evidence-bundle.preserves`
- `release-evidence-bundle-verify.preserves`

Missing or malformed replay index files cause release readback to emit deny receipts rather than trusting logs. Stale or mismatched replay index refs produce diagnostics and deny decisions.

## Bundle signing and export

The replay index Preserves member is part of release bundle members. When signed members are required, the replay index member also requires a valid signed receipt. Release export includes both `replay-evidence-index.preserves` and `replay-evidence-index.signed.preserves`.

## Catalog and MCP

Release artifacts that bind replay indexes classify as `deterministic-replay:release-binding` with `release-dogfood-replay-index:<ref>` text, so existing `search_replay_evidence` read-only MCP search can discover release bindings with `stage=release-binding` or `release-replay-index-ref`.

## Evidence-only boundary

Replay indexes remain readback evidence. They do not grant authority, policy admission, source-gate acceptance, release promotion, provenance trust, resource trust, transport trust, or destructive-operation trust.
