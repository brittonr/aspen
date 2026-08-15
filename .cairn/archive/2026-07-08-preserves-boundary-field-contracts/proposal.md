## Why

The boundary schema adapter validates useful Preserves shape classes, but several classes are still broad: string records do not name allowed vocabularies, ref sequences do not declare uniqueness or non-empty rules, and `AnyRecord` or `AnySequenceRecord` can admit more shape than a trust boundary should accept.

## What Changes

- Add field-level boundary contracts for non-empty strings, stable ids, exact strings, allowed enum values, bounded sequences, unique ref sets, non-empty ref sets, and typed embedded records.
- Replace broad field kinds at high-risk boundaries with the narrowest reviewed field contract that matches the actual semantics.
- Keep semantic gates authoritative for cross-record authority, provenance, policy, resource, transport, replay, and execution decisions.

## Impact

- **Files**: `preserves_rail` boundary field contracts, high-risk schema specs, and boundary validation tests.
- **Testing**: valid fixtures still pass; malformed vocabularies, empty required evidence, duplicate refs, oversized sequences, and over-broad embedded records deny before semantic side effects.
