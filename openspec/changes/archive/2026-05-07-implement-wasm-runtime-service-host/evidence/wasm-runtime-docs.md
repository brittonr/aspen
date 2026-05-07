# WASM runtime docs evidence

- Status: captured
- Change: implement-wasm-runtime-service-host
- Scope: runtime architecture documentation and source anchors for public WASM terminology.

## Files changed

- `docs/runtime-applications.md`
- `openspec/changes/implement-wasm-runtime-service-host/tasks.md`

## Commands

- command: inline Python docs anchor assertion over `docs/runtime-applications.md`
- result: pass; found `WasmRuntimeProfile`, `WasmModule`, ABI version, runner capability/version, deterministic extension, service-fragment, fuel/memory/time/input/output limits, declared host-function bindings, instantiate/call/stop/observe, and `RuntimeHostKind::Wasm`.

- command: `openspec validate implement-wasm-runtime-service-host --strict`
- result: pass; active change validates strictly after docs update.

- command: `git diff --check`
- result: pass; docs/task diff has no whitespace errors.
