# Verification: Octet burn-down, safety `no_unwrap`

Base: `change/sync-finished-tracey-changes-20260924` `d825dc14b` (linear on `origin/molten` `4e31cee55`). Octet: pinned
`octet-toolchain` `fc38f593` (`/nix/store/n8bkxc9p155a27xnx0iqm3l694s8ckkp-cargo-octet-0.1.0`). Private
`CARGO_TARGET_DIR`; commands run under `nice -n 10`, `CARGO_BUILD_JOBS=16`, Nix `--max-jobs 2`.

## Octet

| Scope | Base findings | After | `no_unwrap` (sites) | `no_panic` |
|---|---:|---:|---:|---:|
| workspace (`cargo octet check`) | 3562 | 3481 | 80 (70) → 0 | 1 → 0 |
| `-p molten --lib` | 1493 | 1490 | 3 (3) → 0 | 0 → 0 |

`octet-lint-diff.txt` holds the per-lint diff of both scopes: only `no_unwrap` and `no_panic` moved, and no other family
changed. Both runs remain `warning-only` because of the later families (`octet-root-summary.txt`,
`octet-lib-summary.txt`).

## Repairs

- Integration tests (`tests/content_replication.rs`, `tests/nativesystemextension.rs`,
  `tests/nativesystemextension/support.rs`, `tests/parts/cliharness/p004`, `p011`, `p016`) return `TestResult`. A local
  `OrFail` helper turns a failed `Result`/`Option` step into an error that keeps the old `expect` label and the failure's
  `Debug` form. The `panic!` on an unexpected transfer outcome is now a returned error.
- `src/cluster_harness/fabric_transport.rs`: `fixture_profile` returns the admission `Result`; callers use `?`.
- `src/cluster_harness/runner.rs`: the lifecycle summary builder returns an invalid-harness error for a node without a
  config ref (unreachable when `is_complete`, as before) instead of `expect`.
- `src/wasm/performance/comparison.rs`: `integer_sqrt` halves with `>> 1`. A throwaway `rustc -O` differential run
  (`integer-sqrt-equivalence.txt`) showed the old and new searches equal over `0..200000` and the `u128` edges.

## Rust gates

- `cargo fmt --check`: exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo test --workspace`: exit 0, 2071 passed, 0 failed, 0 ignored.
- The diff adds no `allow` attribute (`git diff | grep '^+.*allow('` → 0 lines). `dylint.toml`, baselines, and
  quarantine files are unchanged.

## Flake checks

`nix build .#checks.x86_64-linux.<check>` passed on this tree for the touched surfaces:
`native-system-extension-host-profile`, `native-system-extension-octet-deny-all`, `wasm-component-performance-profile`,
`cap-std-test-workspaces`, and `fabric-port-boundaries`.

## Review and lifecycle

The no-spec plan review (`reviews/no-spec-classification.json`) approves the proposal, design, acceptance, and tasks
with no findings. The proposal, design, and tasks gates pass (`gate-*.json`), and `cairn validate --strict` reports no
issues (`validate-strict.json`).
