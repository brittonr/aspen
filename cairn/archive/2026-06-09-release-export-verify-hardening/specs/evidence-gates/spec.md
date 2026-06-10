# evidence-gates Specification Delta

## ADDED Requirements

### Requirement: Malformed release export archives deny with receipts
r[molten.evidence.release_export.malformed_archive_denies] Release export verification MUST emit a canonical deny receipt, rather than relying on process failure or logs, when an archive is structurally readable but missing its manifest.

#### Scenario: Missing manifest emits deny receipt
- GIVEN a release evidence archive without `release-export-manifest.preserves`
- WHEN an operator runs `molten dogfood release-export-verify`
- THEN Molten emits `release-export-verify-receipt-v1` with decision `deny`
- AND diagnostics identify the missing manifest

### Requirement: Release export member diagnostics
r[molten.evidence.release_export.member_diagnostics] Release export verification MUST diagnose duplicate archive paths, extra unlisted members, missing listed members, stale member refs, and tampered member bytes in the verification receipt.

#### Scenario: Archive member mismatch emits diagnostics
- GIVEN a release evidence archive with duplicate, extra, missing, stale, or tampered members
- WHEN release export verification runs
- THEN the verification receipt has decision `deny`
- AND diagnostics identify the archive member binding problem
