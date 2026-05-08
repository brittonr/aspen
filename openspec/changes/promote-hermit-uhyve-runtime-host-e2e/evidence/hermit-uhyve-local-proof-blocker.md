# Hermit/Uhyve local proof blocker

Collected: 2026-05-08T16:22:26Z session.

## What passed

- `cargo nextest run -p aspen-jobs --test hermit_uhyve_product_path_test --features plugins-vm`
  - 3 passed, 1 ignored/skipped.
  - This proves the new `HermitUhyveWorker` product-path guardrails only; it is not Hermit runtime-host readiness evidence.
- `openspec validate promote-hermit-uhyve-runtime-host-e2e --strict`
- `openspec validate --all --strict --json`
- `git diff --check`

## Real proof prerequisites checked

- `rustc --print target-list | grep hermit` listed `x86_64-unknown-hermit`.
- `nix build .#uhyve --no-link --print-out-paths -L` now succeeds and provides a real Uhyve binary at a Nix store path.
- `cargo test -p aspen-jobs --test hermit_uhyve_product_path_test --features plugins-vm hermit_uhyve_executes_declared_fixture_through_product_orchestration -- --ignored --nocapture` was run with the Nix-built Uhyve binary and a temporary Hermit marker binary, but the Aspen product-path job dead-lettered because Uhyve rejected the temporary binary as an invalid Hermit image.

## Hermit image build probe

A temporary Hermit image probe under `/tmp/aspen-hermit-probe` attempted:

```bash
cargo build --manifest-path /tmp/aspen-hermit-probe/Cargo.toml --target x86_64-unknown-hermit
```

with:

```toml
[target.'cfg(target_os = "hermit")'.dependencies]
hermit = { git = "https://github.com/hermit-os/hermit-rs.git", tag = "hermit-0.13.0" }
```

The first attempt failed because `hermit v0.13.0` shells out to:

```text
rustup target add x86_64-unknown-none
```

and `rustup` was not available in the current Nix shell.

A second probe placed a temporary fake `rustup` shim in `PATH` that made `rustup target add ...` exit successfully. The Hermit kernel build then progressed but failed with:

```text
Error: Could not find llvm-tools component
Maybe the rustup component `llvm-tools` is missing? Install it through: `rustup component add llvm-tools`
```

## Consequence

Do not promote `runtime-host-hermit-gap.ncl` yet. The Uhyve binary is now packaged by `.#uhyve`, but the remaining work is to provide a real Hermit image that Uhyve accepts and that prints `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED`, then rerun the ignored proof test through Aspen `JobManager`/`WorkerPool` orchestration.
