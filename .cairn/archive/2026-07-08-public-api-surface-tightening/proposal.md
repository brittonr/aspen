## Why

The root library currently exposes many compatibility aliases and broad module surfaces. A wide public API makes internal refactors expensive because implementation details can become accidental compatibility promises.

## What Changes

- Define a small intentional public API surface for stable consumers.
- Keep compatibility aliases during migration, but classify them as stable, deprecated, internal, or temporary.
- Prefer `pub(crate)` for implementation details and re-export only reviewed boundary types/functions.
- Add API inventory and compile checks so public surface changes are deliberate.

## Impact

This change reduces accidental coupling for downstream code and makes future module/crate extraction safer. It should not remove existing public paths in the first slice unless a separate compatibility decision owns that break.
