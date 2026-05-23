# Tasks: Avoid VMCI Nix Store FD Pressure

## Phase 1: Boundary and Classification

- [x] [serial] Add or update diagnostics classification for VMCI Nix source/store FD pressure using the latest medium stderr shape, with redaction tests for tickets, secret keys, env values, full argv, and unbounded paths. (Added `nix_source_store_fd_pressure` class/evidence, exact latest stderr-shape classifier test, and `/nix/store/*-source` subpath redaction while preserving the bounded store-path handle.)
- [x] [parallel] Update dogfood receipt/diagnosis evidence so a failed medium reports the boundary as post-command Nix source/store materialization rather than route, source blob, workspace, timeout, or generic build failure. (Classifier now requires `Too many open files in system` plus Nix source/store context after command progress; `is_post_registration()` includes the new class.)
- [x] [parallel] Record the latest failing medium receipt/log in this change's evidence file and in the superseding VMCI execution-stalls evidence, including `format-check=passed`, `build-cli=failed`, and the `/nix/store/...-source` FD-pressure signature.

## Phase 2: VMCI-safe Nix input strategy

- [x] [serial] Audit VMCI Nix command/config/store mounts to identify why guest Nix still copies or chmod-walks the `nixpkgs` source path through the problematic boundary after public inputs remain fetcher-locked. (Guest `/nix/store` is an overlay with host `/nix/.ro-store` served by virtiofsd as lower layer; even when `nixpkgs` stays fetcher-locked, the actual `nix build` writes/chmod-walks source paths through the overlay-backed store and re-enters virtiofsd.)
- [x] [depends:Phase 2 audit] Implement a VMCI-safe public-input strategy that keeps large public source inputs guest-local/cache-native or streamed through a bounded cache path instead of host virtiofs tree traversal. (VMCI workers now set `ASPEN_CI_NIX_LOCAL_STORE_ROOT=/tmp/aspen-ci-nix-store`; Nix command construction injects `--store local?root=/tmp/aspen-ci-nix-store --option min-free 0 --option max-free 0`, keeping command-fetched public sources and outputs in guest tmpfs/local Nix store rather than the overlay/virtiofs `/nix/store`.)
- [x] [depends:Phase 2 audit] Preserve selective private/offline input rewriting for `tigerstyle`/Octet and `ucan-src`, including correct `narHash`, compatible `original`, and no broad public input path rewrites. (Selective rewrite allowlist remains private/offline-only: `tigerstyle` and `ucan-src`; public/cacheable inputs like `nixpkgs` are not path-rewritten.)
- [x] [parallel] Add deterministic unit tests for input classification, command/config construction, and broad-rewrite prevention. (Added local-store flag tests alongside existing selective rewrite tests.)

## Phase 3: Layered proof

- [x] [depends:Phase 2] Run `nix run .#rustfmt`, targeted Rust tests for the executor/dogfood/diagnostic changes, `cargo test -p aspen-dogfood vmci -- --nocapture`, and `openspec validate avoid-vmci-nix-store-fd-pressure --strict`. (Validated after the local-store/UCAN rewrite patches; timeout tuning plus the rail wait-budget guard were validated with targeted dogfood tests and strict OpenSpec validation.)
- [x] [depends:Phase 3 tests] Re-run `nix run .#dogfood-local-vmci-medium` from a clean prechecked process/disk state and archive the receipt/log path. (Latest successful rerun `proc_610989a9d21b`; log `target/runtime-proof/vmci-medium-20260523T001418Z.log`; receipt `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260523T002711Z.json`; CI run `ef06231a-3b09-4ac0-a749-7272ad97014b`; stages `check/format-check`, `cache-warm/build-cli-deps`, and `build/build-cli` all passed.)
- [x] [depends:medium receipt] If medium passes or fails at a new non-FD boundary, update evidence and decide whether to escalate to clippy/full; if it still fails with VMCI Nix source/store FD pressure, identify the exact remaining materialization path before attempting another fix. (Medium now passes with guest-local store configuration; no VMCI Nix source/store FD-pressure signature observed.)

## Phase 4: Integration with existing VMCI changes

- [x] [depends:medium receipt] Update `resolve-vmci-nix-build-execution-stalls` tasks/evidence to point at this change for the source/store FD-pressure fix. (Cross-change evidence now points at the successful medium receipt/log above.)
- [x] [depends:medium receipt] Re-run strict OpenSpec validation for this change and the related active VMCI changes before implementation is considered ready to land. (Validated strictly after the successful medium proof: `avoid-vmci-nix-store-fd-pressure`, `resolve-vmci-nix-build-execution-stalls`, `add-vmci-layered-harness`, `fail-fast-direct-only-route-loss`, `propagate-rpc-ci-source-archive`, and `debug-vmci-ci-workspace-blob-stall`.)
