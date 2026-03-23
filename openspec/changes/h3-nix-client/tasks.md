## 1. Nix packaging

- [x] 1.1 Add `ci-aspen-h3-proxy` package to `flake.nix` — build `aspen-h3-proxy` binary from `ciCommonArgs`
- [x] 1.2 Add `ci-aspen-nix-cache-gateway-h3` package to `flake.nix` — build gateway with `--features h3-serving` from `ciCommonArgs`
- [x] 1.3 Verify both packages build: `nix build .#ci-aspen-h3-proxy` and `nix build .#ci-aspen-nix-cache-gateway-h3`

## 2. VM integration test

- [x] 2.1 Create `nix/tests/nix-cache-h3-test.nix` — single-machine test: start aspen-node, start gateway with `--h3`, parse endpoint ID from journal, start h3-proxy, verify nix-cache-info through proxy
- [x] 2.2 Wire test into `flake.nix` checks as `nix-cache-h3-test`
- [x] 2.3 Run the VM test and fix any issues: `nix build .#checks.x86_64-linux.nix-cache-h3-test`

## 3. Verification and cleanup

- [x] 3.1 Run existing `nix-cache-gateway-test` to verify no regressions in the TCP path (TCP gateway unchanged — clippy-only change to h3 codepath)
- [x] 3.2 Mark iroh-upgrade-097 tasks 4.4 and 5.7 as complete
- [x] 3.3 Run `cargo clippy -p aspen-h3-proxy -p aspen-nix-cache-gateway --features h3-serving -- --deny warnings`
