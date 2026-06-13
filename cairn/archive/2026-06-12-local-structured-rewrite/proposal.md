## Why

Molten now has immutable local artifacts, schema identities, typed-storage migration recipes, evaluation-cache entries, executable transcripts, and upgrade-session receipts. The next local operation is safe structural rewrite: find canonical artifact substructures, preview the exact immutable replacements, and apply them by installing new artifacts rather than mutating old ones.

Text-only search/replace would bypass Molten's artifact, policy, and evidence boundaries. Local structured rewrite keeps matching and replacement bounded to canonical Preserves values, emits receipts, and leaves name cutover to upgrade sessions.

## What Changes

- Add canonical local rewrite query, match, diff, plan, and receipt records.
- Add bounded pattern matching over artifact kind, Preserves record labels, exact/contained string values, schema shape kinds, and explicit refs.
- Add visibility filtering for hidden/unauthorized refs before returning matches.
- Add dry-run preview with canonical structural diffs and dependency impact sets.
- Add apply that installs new immutable artifact records and preserves old artifact payloads.
- Add an upgrade-session hook that turns applied rewrites into receipt-backed upgrade plan tasks.
- Add CLI commands under `molten test rewrite find|preview|apply|show`.

## Impact

This gives Molten a local, evidence-bearing refactor substrate over the artifact registry. Future slices can add richer schema/protocol patterns, catalog/MCP views, transcript gate selection, and migration-recipe generation on top of this core without weakening immutability or policy boundaries.
