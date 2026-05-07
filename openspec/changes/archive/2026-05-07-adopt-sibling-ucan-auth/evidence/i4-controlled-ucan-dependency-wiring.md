# I4 controlled Cargo/Nix UCAN dependency wiring

- Change: `adopt-sibling-ucan-auth`
- Task: Add controlled Cargo/Nix wiring for `../ucan` / `../ucan/crates/ucan-core` with documented local development and reproducible fallback.
- Started: 2026-05-06T23:32:37Z
- Completed: 2026-05-06T23:39:39Z
- Status: captured and locally verified

## Wiring changes

| File | Change |
| --- | --- |
| `Cargo.toml` | Added workspace dependencies `ucan` and `ucan-core`, pinned to sibling commit `ad61b53e89fa45f9bf7d313ce14c45de645bf53d` via `ssh://git@github.com/brittonr/ucan.git`. |
| `crates/aspen-auth-core/Cargo.toml` | Added `ucan-core = { workspace = true }`; keeps portable dependency on sibling no-std core only. |
| `crates/aspen-auth/Cargo.toml` | Added `ucan = { workspace = true }`; root UCAN shell remains runtime-only. |
| `.cargo/config.toml` | Added `net.git-fetch-with-cli = true` so private SSH git dependencies use the operator's SSH agent/config. Added commented local patch instructions for `../ucan`. |
| `flake.nix` | Added `ucan-src` flake input pinned to the same commit and wired `overrideVendorGitCheckout` branches to replace Cargo's git checkout with that locked source during Nix vendoring. |
| `flake.lock` | Recorded locked `ucan-src` input. |
| `Cargo.lock` | Recorded `ucan`, `ucan-core`, `verified-logic`, and transitive verified-logic/Verus dependencies. |

## Boundary policy encoded

- Portable Aspen core path: `aspen-auth-core -> ucan-core` only.
- Runtime Aspen shell path: `aspen-auth -> ucan` and inherits root UCAN shell semantics including signer/resolver/proof/revocation/replay surfaces.
- Local development path is opt-in and commented to avoid breaking machines without `../ucan`:

```toml
# [patch."ssh://git@github.com/brittonr/ucan.git"]
# ucan = { path = "../ucan" }
# ucan-core = { path = "../ucan/crates/ucan-core" }
```

## Verification commands

```text
CARGO_TARGET_DIR=target/agent cargo metadata --format-version 1 >/tmp/aspen-metadata-ucan.json
```

- Result: PASS.
- Evidence: Cargo fetched `ssh://git@github.com/brittonr/ucan.git` at `ad61b53e...` and added `ucan`, `ucan-core`, and `verified-logic` lock entries.

```text
CARGO_TARGET_DIR=target/agent cargo check -p aspen-auth-core --no-default-features
```

- Result: PASS.
- Evidence: completed `aspen-auth-core v0.1.0`; `ucan-core v0.1.0` compiled from the pinned git source.

```text
CARGO_TARGET_DIR=target/agent cargo check -p aspen-auth --all-targets
```

- Result: PASS.
- Evidence: completed `aspen-auth v0.1.0`; `ucan v0.1.0` and `verified-logic v0.1.0` compiled from the pinned git source.

```text
nix flake lock --option allow-import-from-derivation true
```

- Result: PASS.
- Evidence: added `ucan-src` locked to `git+ssh://git@github.com/brittonr/ucan.git` at `ad61b53e...`.

```text
nix flake metadata --json | python -c '... print ucan-src locked source ...'
```

- Result: PASS.
- Output: `git ssh://git@github.com/brittonr/ucan.git ad61b53e89fa`.

```text
CARGO_TARGET_DIR=target/agent cargo metadata --locked --format-version 1 >/tmp/aspen-metadata-locked-ucan.json
```

- Result: PASS.
- Evidence: locked Cargo graph resolves with the new UCAN dependencies.

## Known release/CI failure mode

Both Cargo and Nix pins currently use SSH access to `git@github.com:brittonr/ucan.git`. Reproducible builds have a fixed commit and flake lock, but machines without GitHub SSH credentials will fail at source fetch. That is the documented current failure mode until the sibling UCAN repository is made public or mirrored into an Aspen-controlled source archive/cache.
