# Design: Release evidence export bundle

## Overview

`molten dogfood release-export` reads a realized dogfood output after promotion summary generation, constructs a canonical `release-export-manifest-v1`, writes it to `--manifest-out`, and writes a deterministic `tar.zst` archive to `--out`. The archive stores the manifest plus the listed payload files using deterministic tar metadata.

`molten dogfood release-export-verify` reads the archive, recomputes payload member refs, parses the embedded manifest, and emits `release-export-verify-receipt-v1` with `pass` only when archive payload refs match the manifest exactly.

## Evidence model

The manifest records:

- the realized output path ref,
- the promotion summary ref,
- deterministic member path/content refs,
- checks for promotion summary pass, member binding, deterministic layout, evidence-only status, and no release authority.

The verify receipt records:

- pass/deny decision,
- manifest ref,
- promotion summary ref,
- diagnostics for stale, missing, extra, or tampered members,
- evidence-only and no-authority checks.

## Boundary

The archive is a review/export artifact only. Verification of archive member refs does not grant release authority, source trust, Octet trust, Cairn trust, Nix trust, keyring authority, retention authority, transport trust, or destructive-operation clearance.
