# Repository Guidelines

## Project Structure & Module Organization

- `src/` houses the core Aspen runtime and binaries.
- `crates/` contains internal libraries (e.g., networking, storage, TUI).
- `tests/` holds integration and madsim-based tests; `benches/` for benchmarks.
- `examples/`, `docs/`, and `fuzz/` provide sample usage, design notes, and fuzz targets.
- `scripts/` includes helper tooling; `openraft/` is a vendored dependency.

## Build, Test, and Development Commands

- `nix develop`: enter the pinned Rust/Nix dev shell.
- `nix develop -c cargo build`: build all crates and binaries.
- `nix develop -c cargo nextest run`: run the full test suite.
- `nix develop -c cargo nextest run -P quick`: faster test profile.
- `nix fmt`: format Rust and Nix sources.
- `nix develop -c cargo clippy --all-targets -- --deny warnings`: lint with Clippy.
- `nix run .#bench`: run benchmarks.

## Coding Style & Naming Conventions

- Rust code uses 4-space indentation (rustfmt default).
- Run `nix fmt` before committing; `rustfmt.toml` defines formatting rules.
- Follow Rust naming: `snake_case` for functions/modules, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Prefer explicit error handling and fail-fast behavior (see `tigerstyle.md`).

## Testing Guidelines

- Primary frameworks: `cargo-nextest`, `madsim`, `tokio::test`, and `proptest`.
- Integration tests live in `tests/`; simulation artifacts are stored in `docs/simulations/`.
- Add tests with descriptive names matching the subsystem (e.g., `tests/raft_*`).

## Commit & Pull Request Guidelines

- Use Conventional Commit style: `feat:`, `fix:`, `test:`, `docs:` (see `git log`).
- Keep commits scoped and imperative (e.g., `feat: add pub/sub router`).
- PRs should include a concise summary, linked issues, and test commands/results.
- Add screenshots or TUI recordings when UI changes are involved.

## Configuration Notes

- Feature flags are defined in `Cargo.toml`; enable via `--features sql,forge`.
- Use `nix develop` to ensure the Rust 2024 toolchain and dependencies match CI.
