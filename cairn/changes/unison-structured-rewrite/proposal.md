## Why

Molten will need to update schemas, policies, protocol manifests, effect manifests, docs, and storage migration recipes. Text search/replace over source files is too imprecise for artifact-bound runtime state and can accidentally change comments, strings, or unrelated names.

Unison's structural find and replace points to a better pattern: query and rewrite code by structure. Molten should provide structured rewrites over canonical artifacts and registry metadata, with preview, validation, and receipt-backed application.

## What Changes

- Add structured query and rewrite plans over Molten artifacts, schemas, policies, manifests, envelopes, and docs.
- Match canonical Preserves/schema/manifest structures rather than raw text whenever possible.
- Support dry-run preview, impacted-artifact analysis, policy admission, validation, transcript reruns, and upgrade-session integration.
- Treat rewrites as producing new immutable artifacts and metadata moves, never mutating existing artifact content in place.
- Emit receipts for query, preview, rewrite plan admission, artifact creation, validation, and application.

## Impact

Molten upgrades become safer and more auditable. The first milestone can implement structural find over artifact metadata and a simple rewrite that creates new schema or manifest artifacts, then validates affected transcripts before proposing cutover.
