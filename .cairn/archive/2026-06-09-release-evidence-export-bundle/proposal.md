# Change: Release evidence export bundle

## Summary

Add an operator-facing release export command that packages the realized dogfood release evidence graph into a deterministic portable archive with a canonical manifest and verification receipt.

## Motivation

Promotion summary readback proves the final evidence-only decision can be read and verified in place, but operators still need a single portable artifact for release review handoff. The export must bind archive members by content refs, verify round trips without trusting logs, and preserve the evidence-only boundary.

## Scope

- Add CLI UX for exporting a realized dogfood output to `release-evidence.tar.zst`.
- Emit `release-export-manifest-v1` and `release-export-verify-receipt-v1` Preserves artifacts.
- Include the dogfood, Nix, bundle, promotion, summary, signed receipts, text summaries, and signed keyring members.
- Keep the archive and receipts evidence-only; they do not grant release authority or replace subsystem gates.

## Non-Goals

- Publishing releases.
- Granting trust to source, Octet, Cairn, Nix, keyring, retention, transport, or destructive-operation subsystems.
- Replacing ledger/keyring verification with archive membership.
