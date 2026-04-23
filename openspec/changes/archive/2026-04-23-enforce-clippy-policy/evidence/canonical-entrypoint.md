# Canonical lint entrypoint

The canonical checked-in lint entrypoint for the rollout scope is:

```bash
./scripts/clippy-policy.sh all
```

Subcommands:

- `./scripts/clippy-policy.sh clippy`
- `./scripts/clippy-policy.sh rustdoc`

Why this satisfies the policy:

- lives in the repository checkout
- does not depend on shell aliases or editor integrations
- pins the rollout scope explicitly to `aspen-layer` and `aspen-time`
- bakes in `-D warnings` for Clippy and `RUSTDOCFLAGS="-D warnings"` for rustdoc
- normalizes `TMPDIR`, `CARGO_INCREMENTAL`, and `RUSTC_WRAPPER` for this environment
