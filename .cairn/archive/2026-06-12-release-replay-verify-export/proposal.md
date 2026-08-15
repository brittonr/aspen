# Change: release-replay-verify-export

## Summary

Bind the raw deterministic replay verify receipt alongside replay indexes in dogfood release evidence outputs, signed release bundles, and release export archives.

## Motivation

Replay indexes make release readback discoverable, but operators also need the individual generic replay verify receipt available as reusable evidence in the realized dogfood output and portable release archive. The verify receipt remains evidence-only and does not replace release authority, source gates, policy, provenance, Octet, Cairn, signed keyring, or promotion checks.

## Scope

- Add a local dogfood output for the generic replay verify receipt.
- Include and sign the replay verify member in release bundle verification paths.
- Require release readback to bind replay verify refs consistently with the replay index.
- Include replay verify members in deterministic release export archives.
