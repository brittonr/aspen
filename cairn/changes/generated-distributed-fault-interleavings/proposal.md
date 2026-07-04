## Why

Named distributed simulation fixtures cover important fault classes, but regressions often appear in combinations: duplicate after restart, partition during stale evidence, reorder with retry, or resource pressure while a quorum command is in flight. Manually enumerating those combinations does not scale and makes missing coverage hard to see.

## What Changes

- Add bounded Hegel-generated distributed simulation properties for topology, commands, scheduler profiles, and fault-plan interleavings.
- Preserve deterministic seeds and minimized fault plans for failing generated cases.
- Promote interesting generated failures into named direct fixtures when they represent a new invariant or regression class.
- Add negative generated coverage proving invalid evidence, missing authority, corrupted receipts, partitioned quorum, and undeclared ambient state deny before side effects.

## Impact

The simulation layer becomes better at finding unexpected interactions while retaining reproducible review evidence. Generated failures become actionable because the seed, topology, fault plan, and receipt refs are bound into repro artifacts.