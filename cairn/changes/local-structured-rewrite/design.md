## Context

The implementation-oriented local slice narrows the broader `unison-structured-rewrite` idea to Molten's current local artifact registry. Unison's structured refactor workflow is prior art only; Molten does not adopt UCM syntax, hash formats, typechecker behavior, or codebase model.

## Goals

- Query immutable local artifact payloads through bounded canonical Preserves patterns.
- Keep paths, matches, diffs, plans, and receipts content-addressed and deterministic.
- Apply visibility filters before returning matches.
- Preview rewrites before side effects and include reverse-dependency impact sets.
- Apply rewrites only by installing new immutable artifact records.
- Preserve old artifacts and leave name/channel cutover to upgrade-session tasks.
- Bind transcript and schema-migration hooks as refs without inventing ambient authority.

## Non-Goals

- No arbitrary user code or host shell transformations.
- No in-place artifact mutation.
- No text-only replacement that bypasses Preserves parsing and artifact validation.
- No distributed rewrite coordination in this slice.
- No automatic name cutover; upgrade sessions remain responsible for cutover.

## Records

`rewrite-query-v1` binds scope roots, dependency expansion, artifact kinds, a bounded pattern, policy/capability refs, hidden refs, and checks.

`rewrite-match-v1` binds artifact ref, kind, payload ref, stable canonical paths, and value refs for matched bindings.

`rewrite-diff-v1` binds old artifact ref, old/new payload refs, changed paths, preview text, and structural rewrite checks.

`rewrite-plan-v1` binds planner/capability refs, query value/ref, replacement, matches, diffs, impact refs, transcript refs, schema migration refs, policy refs, and checks.

`rewrite-receipt-v1` is emitted for query, preview, and apply. Apply receipts bind the preview ref, installed new artifact refs, and install receipt refs.

## Matching subset

The first subset supports:

- `any`
- `artifact-kind`
- `record-label`
- `string-equals`
- `string-contains`
- `schema-shape-kind`
- `ref-contains`

The apply path currently supports exact structural string-value replacement. Records and sequences are rebuilt canonically. Sets/dictionaries are intentionally not rewritten in this slice.

## Upgrade hook

After apply, `upgrade_plan_from_apply` produces an ordinary `upgrade-plan-v1` with `install-artifact` tasks for old/new artifact pairs and a transcript gate task that binds the rewrite preview/apply receipts. The plan does not move names automatically.
