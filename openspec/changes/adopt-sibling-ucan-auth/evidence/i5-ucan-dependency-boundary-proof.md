# I5 UCAN dependency-boundary proof

- Change: `adopt-sibling-ucan-auth`
- Task: Prove dependency boundaries with `cargo tree`/feature checks for `aspen-auth-core`, `aspen-auth`, and protected `aspen-core --no-default-features` paths.
- Started: 2026-05-06T23:40:53Z
- Completed: 2026-05-06T23:42:08Z
- Status: PASS

## Commands and results

### `aspen-auth-core` normal dependency tree

```text
CARGO_TARGET_DIR=target/agent cargo tree -p aspen-auth-core --no-default-features --edges normal --prefix none > /tmp/aspen-auth-core-tree.txt
```

Filtered output:

```text
aspen-auth-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth-core)
ucan-core v0.1.0 (ssh://git@github.com/brittonr/ucan.git?rev=ad61b53e89fa45f9bf7d313ce14c45de645bf53d#ad61b53e)
```

Assertion result:

- PASS: `aspen-auth-core` includes `ucan-core`.
- PASS: `aspen-auth-core` excludes root `ucan`.
- PASS: `aspen-auth-core` excludes `verified-logic`.

### `aspen-auth` normal dependency tree

```text
CARGO_TARGET_DIR=target/agent cargo tree -p aspen-auth --edges normal --prefix none > /tmp/aspen-auth-tree.txt
```

Filtered output:

```text
aspen-auth v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth)
aspen-auth-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth-core)
ucan-core v0.1.0 (ssh://git@github.com/brittonr/ucan.git?rev=ad61b53e89fa45f9bf7d313ce14c45de645bf53d#ad61b53e)
ucan v0.1.0 (ssh://git@github.com/brittonr/ucan.git?rev=ad61b53e89fa45f9bf7d313ce14c45de645bf53d#ad61b53e)
verified-logic v0.1.0 (ssh://git@github.com/brittonr/ucan.git?rev=ad61b53e89fa45f9bf7d313ce14c45de645bf53d#ad61b53e)
```

Assertion result:

- PASS: `aspen-auth` includes root `ucan`.
- PASS: `aspen-auth` includes `ucan-core` through both Aspen core and root UCAN paths.
- PASS: `aspen-auth` includes `verified-logic` only through root UCAN shell.

### Protected `aspen-core --no-default-features` normal tree

```text
CARGO_TARGET_DIR=target/agent cargo tree -p aspen-core --no-default-features --edges normal --prefix none > /tmp/aspen-core-nodefault-tree.txt
```

Filtered output:

```text
aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
```

Assertion result:

- PASS: protected `aspen-core --no-default-features` excludes `aspen-auth`.
- PASS: protected `aspen-core --no-default-features` excludes `ucan` and `ucan-core`.
- PASS: protected `aspen-core --no-default-features` excludes `verified-logic`.

### Feature tree checks

```text
CARGO_TARGET_DIR=target/agent cargo tree -p aspen-auth-core --no-default-features -e features --prefix none > /tmp/aspen-auth-core-features.txt
CARGO_TARGET_DIR=target/agent cargo tree -p aspen-core --no-default-features -e features --prefix none > /tmp/aspen-core-nodefault-features.txt
```

Filtered output showed only `aspen-auth-core`/`ucan-core` for the auth-core path and only `aspen-core` for the protected core path among UCAN/auth terms.

### Deterministic no-std boundary checker

```text
python scripts/check-aspen-core-no-std-boundary.py \
  --manifest-path crates/aspen-core/Cargo.toml \
  --allowlist scripts/aspen-core-no-std-transitives.txt \
  --output /tmp/aspen-core-no-std-current-ucan.txt \
  --diff-output /tmp/aspen-core-no-std-diff-ucan.txt
```

- Result: PASS, exit 0.

## Summary

The UCAN dependency split is currently bounded as intended:

- `aspen-auth-core` depends on sibling no-std `ucan-core` only.
- root `ucan` and `verified-logic` remain runtime-shell dependencies under `aspen-auth`.
- protected `aspen-core --no-default-features` remains free of Aspen auth and UCAN dependencies.
