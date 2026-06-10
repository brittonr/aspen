# Change: Release export verification hardening

## Summary

Harden release export verification diagnostics so malformed portable archives produce deny receipts for missing manifests, duplicate paths, extra members, and tampered payloads instead of relying on command failure or vague count mismatches.

## Motivation

The release export archive is a handoff artifact. Operators need fail-closed readback that explains archive defects as canonical evidence, including archives that omit the manifest or contain duplicate/extra/tampered members.

## Scope

- Make `release-export-verify` emit deny receipts for missing archive manifests.
- Diagnose duplicate archive member paths.
- Diagnose extra unlisted members and tampered listed members.
- Add CLI regression coverage.

## Non-Goals

- Trusting the archive as release authority.
- Publishing releases.
- Replacing source, Octet, Cairn, Nix, keyring, or subsystem gates.
