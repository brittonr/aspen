# WASM portable model/admission evidence

- Status: captured
- Change: implement-wasm-runtime-service-host
- Scope: portable runtime-core WASM host boundary, admission, pure instantiation/call/stop/observe helpers, and redacted receipts.

## Files changed

- `crates/aspen-runtime-core/src/lib.rs`

## Commands

- command: `rustfmt crates/aspen-runtime-core/src/lib.rs`
- result: pass; formatted the touched Rust source.

- command: `CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core wasm --all-targets`
- result: pass; 4 WASM-focused tests passed, covering service-core vocabulary alignment, positive instantiate/call/stop/observe lifecycle, fail-closed admission, and receipt redaction.

- command: `openspec validate implement-wasm-runtime-service-host --strict`
- result: pass; active change validates strictly.

- command: `git diff --check`
- result: pass; no whitespace errors in the implementation diff.

## Coverage notes

The implementation adds data-only WASM DTOs and pure validation helpers. It does not claim a live WASM engine or scheduler. Admission fails closed before exposing handles when the host kind or artifact is wrong, module identity/ABI/entrypoint do not match, runner capability is absent, resources or input/output limits exceed policy, host functions are undeclared, deterministic mode requests ambient/unsafe functions, service fragments omit route declarations, or output/diagnostic receipt material contains raw secrets.
