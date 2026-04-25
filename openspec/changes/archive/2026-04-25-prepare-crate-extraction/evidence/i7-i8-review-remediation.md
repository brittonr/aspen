# I7/I8 review remediation

This artifact records the corrective action after review found that I7/I8 were treated as verified while the task gate was still failed.

## Dependency-boundary evidence restored

The dependency tree artifacts now include both the exact cargo tree command and a forbidden-dependency grep audit ending in `forbidden_dependency_matches=none` and `exit_code=0`:

- `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-types-dependency-tree.txt`
- `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-facade-dependency-tree.txt`

The forbidden audit strips path suffixes from `cargo tree` package lines before matching, so repository paths like `/home/.../aspen` and vendor warning paths cannot mask or create package-level dependency matches.

## I7 transition plan tightened

`docs/crate-extraction/aspen-raft-kv-types.md` now records concrete subsystem owner status and per-consumer decisions for:

- dependency-key alias use
- temporary compatibility crate / re-export use
- verification rail
- removal criteria

No row is left with `owner needed` in the `aspen-raft-types` package transition table.

## Task status correction

I7 and I8 are intentionally unchecked again in `tasks.md` because the captured `openspec_gate tasks` transcript remains failed. The code, docs, and evidence stay in the repo as prepared work, but the OpenSpec task status is not treated as verified until the gate blockers are closed.
