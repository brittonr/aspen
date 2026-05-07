# Hermit final validation

- Change: `implement-hermit-unikernel-profile`
- Task: focused tests, strict OpenSpec validation, helper verification, whitespace checks
- Started: `2026-05-07T02:22:40Z`
- Completed: `2026-05-07T02:23:43Z`

## Verification commands

- `rustfmt crates/aspen-runtime-core/src/lib.rs --check`
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core --all-targets`
- Python docs source-anchor assertion for Hermit public terminology.
- `openspec validate implement-hermit-unikernel-profile --strict`
- `python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify implement-hermit-unikernel-profile --json`
- `git diff --check`

## Result

- Runtime-core tests passed: 20 passed, including 4 focused Hermit tests.
- Strict OpenSpec validation passed.
- Helper verification was run before this final task was marked complete and reported only the expected in-progress task warning; it is re-run after task closure before archive.
- Whitespace checks passed.
