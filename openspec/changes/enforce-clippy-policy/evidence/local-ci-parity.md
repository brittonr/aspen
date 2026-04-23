# Local and CI lint parity

## Canonical local command

```bash
./scripts/clippy-policy.sh all
```

## CI semantics

CI must invoke `./scripts/clippy-policy.sh all` directly, or an equivalent checked-in wrapper that preserves these exact semantics:

- `cargo clippy -p aspen-layer -p aspen-time --all-targets -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p aspen-layer -p aspen-time`
- `env -u CARGO_INCREMENTAL RUSTC_WRAPPER=` and explicit `TMPDIR` handling from the script

The checked-in script is the source of truth for rollout-scope lint semantics; CI must not add a stricter hidden variant for the same scope.
