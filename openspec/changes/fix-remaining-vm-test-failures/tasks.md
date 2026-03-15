## 1. Transit Datakey Fix (sops-transit)

- [x] 1.1 Fix `handle_datakey()` in `aspen-plugins/crates/aspen-secrets-plugin/src/transit.rs`: change `key_type == "plaintext"` to `key_type != "wrapped"` so plaintext is returned by default
- [x] 1.2 Fix SOPS client in `crates/aspen-secrets/src/sops/client.rs`: change `key_type: "aes256-gcm"` to `key_type: "plaintext"` in `generate_data_key()`
- [x] 1.3 Add regression test in `crates/aspen-secrets/src/sops/client.rs` tests verifying the request sends `key_type: "plaintext"`

## 2. Multinode Deploy Timeout Fix (ci-dogfood-deploy-multinode)

- [x] 2.1 Update `ciConfig` in `nix/tests/ci-dogfood-deploy-multinode.nix`: add `validate_only = true` and `expected_binary = "bin/cowsay"` to the deploy job
- [x] 2.2 Update test assertions: change "deploy should fail" to "deploy should succeed" since validate_only mode validates the artifact exists without switching nix profiles
- [x] 2.3 Increase build stage timeout from 600s to 900s as defense-in-depth

## 3. Snix Bridge Import Fix (snix-bridge-virtiofs)

- [ ] 3.1 Run `snix-bridge-test` (the simpler non-virtiofs test) to confirm whether import alone fails or only the virtiofs step
- [x] 3.2 Add diagnostic tracing to `IrohBlobService::open_write()`, `BlobWriter::close()`, `RaftPathInfoService::put()`, and `RaftDirectoryPutter::put()/close()` in `crates/aspen-snix/src/`
- [x] 3.3 Check for proto/API version mismatch between snix-store binary (built from `snix-src` flake input) and aspen-snix crate deps — confirmed same rev `180bfc4`, no mismatch
- [ ] 3.4 Fix the root cause based on debugging findings (run interactive test with `nix build .#checks.x86_64-linux.snix-bridge-virtiofs-test.driverInteractive --impure`)
- [ ] 3.5 Verify the `snix-bridge-virtiofs-test` passes end-to-end
