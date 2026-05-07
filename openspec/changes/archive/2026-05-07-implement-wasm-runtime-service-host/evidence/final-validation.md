# WASM final validation evidence

- Status: captured
- Change: implement-wasm-runtime-service-host
- Scope: final focused validation before archive.

## Commands

- command: `rustfmt crates/aspen-runtime-core/src/lib.rs --check`
- result: pass; touched Rust source is formatted.

- command: `CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core --all-targets`
- result: pass; 30 runtime-core tests passed, including 4 WASM-focused positive/negative tests.

- command: inline Python docs anchor assertion over `docs/runtime-applications.md`
- result: pass; WASM docs anchors present.

- command: `openspec validate implement-wasm-runtime-service-host --strict`
- result: pass; active change validates strictly.

- command: `git diff --check`
- result: pass; no whitespace errors.

- command: `python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify implement-wasm-runtime-service-host --json || true`
- result: expected warning before this final checkbox was marked complete: `tasks incomplete: {'done': 8, 'todo': 1, 'in_progress': 0}`. Rerun after marking this task complete must be clean before archive.
