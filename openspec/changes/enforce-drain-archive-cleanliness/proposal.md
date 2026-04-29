## Why

A recent drain completion was reviewed as suspect because the active change still appeared in reviewer context after archive. Even when the working tree is clean, the drain loop needs durable post-archive evidence proving active paths are gone, archive paths are present, and `.drain-state.md` is removed.

## What Changes

- Add a drain completion check that records active-change listing, archive path, and `.drain-state.md` absence.
- Fail if any non-archive active change directory remains after a drain claims queue empty.
- Require archived verification paths to use archive-relative locations before final preflight.

## Capabilities

### New Capabilities
- `openspec-governance.drain-archive-cleanliness`: Drain completion proves the active queue is empty and archive paths are consistent.

## Impact

- **Files**: drain skill instructions, preflight/archive helper, evidence template.
- **Testing**: Fixture with leftover active path fails; clean archived path passes.
