# Design: Release export verification hardening

## Overview

Archive reading now separates archive diagnostics from manifest parsing. `release-export-verify` can continue to a canonical `release-export-verify-receipt-v1` even when the archive omits `release-export-manifest.preserves`.

The verifier records diagnostics for:

- missing manifest,
- duplicate member paths,
- extra archive payload members not listed in the manifest,
- missing listed members,
- stale or tampered listed member content refs.

## Evidence boundary

A deny receipt remains the primary operator artifact for malformed archives. The archive, manifest, and verify receipt remain evidence-only and do not grant release publication authority or subsystem trust.
