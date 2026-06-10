# evidence-gates Specification Delta

## ADDED Requirements

### Requirement: Release export manifest
r[molten.evidence.release_export.manifest] Release evidence export manifests MUST bind a realized dogfood output path, the release promotion summary ref, deterministic member path/content refs, and evidence-only/no-authority checks in canonical Preserves.

#### Scenario: Manifest binds portable release evidence members
- GIVEN a realized dogfood output with pass promotion summary evidence
- WHEN an operator creates a release export manifest
- THEN the manifest records the output path ref, promotion summary ref, member refs, deterministic layout check, evidence-only check, and no-release-authority check

### Requirement: Release export archive
r[molten.evidence.release_export.archive] Release evidence archives MUST use deterministic member ordering and file metadata so the same manifest and member bytes produce a stable portable review artifact.

#### Scenario: Deterministic archive export
- GIVEN a release export manifest and its listed members
- WHEN an operator writes the release evidence archive
- THEN the archive contains the manifest and listed payload members with deterministic tar metadata and without using logs as primary evidence

### Requirement: Release export verification
r[molten.evidence.release_export.verify] Release export verification MUST recompute member refs from the archive and emit a pass/deny receipt instead of relying on archive command logs.

#### Scenario: Tampered export denies
- GIVEN a release evidence archive with a missing, extra, stale, or tampered payload member
- WHEN an operator verifies the archive
- THEN verification emits `release-export-verify-receipt-v1` with decision `deny` and diagnostics identifying the member binding failure

### Requirement: Release export dogfood
r[molten.evidence.release_export.dogfood] The Nix dogfood release check MUST emit a portable release evidence archive, manifest, and verification receipt while preserving the evidence-only boundary.

#### Scenario: Dogfood emits portable export evidence
- GIVEN the dogfood release flow has produced signed promotion and promotion summary evidence
- WHEN the Nix dogfood check completes
- THEN it emits `release-evidence.tar.zst`, `release-export-manifest.preserves`, and `release-export-verify.preserves` with pass verification and no release authority granted
