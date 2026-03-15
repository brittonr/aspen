## 1. Remove keyservice bridge from aspen-sops

- [x] 1.1 Delete `crates/aspen-sops/src/keyservice/` directory and `crates/aspen-sops/proto/keyservice.proto`
- [x] 1.2 Remove `keyservice` feature flag, tonic/tonic-build/prost deps from `crates/aspen-sops/Cargo.toml`
- [x] 1.3 Remove keyservice-related code from `cli.rs`, `main.rs`, `lib.rs`, and `build.rs`
- [x] 1.4 Remove Go SOPS interop subtests and `pkgs.sops` from `nix/tests/sops-transit.nix`
- [x] 1.5 Verify `cargo build -p aspen-sops` and existing sops unit tests pass

## 2. Scaffold aspen-sops-install-secrets crate

- [x] 2.1 Create `crates/aspen-sops-install-secrets/` with `Cargo.toml` (binary crate, deps: aspen-secrets, serde, serde_json, clap, nix, tokio)
- [x] 2.2 Define manifest structs: `Manifest`, `SecretEntry`, `TemplateEntry`, `LoggingConfig`, `FormatType` with serde derives matching Go schema
- [x] 2.3 Implement CLI arg parsing: positional manifest path, `-check-mode` flag, `--ignore-passwd` flag
- [x] 2.4 Write unit tests for manifest deserialization with sample Go-produced JSON

## 3. Core decryption pipeline

- [x] 3.1 Implement `decrypt_secret()`: detect format, call `aspen_secrets::sops::decrypt::decrypt_file()`, extract key path from decrypted output
- [x] 3.2 Implement YAML key extraction: traverse nested keys by `/`-separated path (matching Go's `recurseSecretKey`)
- [x] 3.3 Implement JSON key extraction: same traversal for JSON documents
- [x] 3.4 Handle binary format: return raw decrypted bytes without parsing
- [x] 3.5 Implement SOPS file caching: decrypt each unique sopsFile once, reuse for multiple secrets from the same file
- [x] 3.6 Write unit tests for key extraction (nested paths, empty key, missing key errors)

## 4. Age key management

- [x] 4.1 Implement age key file loading from manifest's `ageKeyFile` path
- [x] 4.2 Implement SSH-to-age conversion for ed25519 keys from `ageSshKeyPaths` (use `ssh-key` crate to parse, convert to age identity)
- [x] 4.3 Write combined age key file to `<secretsMountPoint>/age-keys.txt` and set `SOPS_AGE_KEY_FILE` env var
- [x] 4.4 Write tests for SSH-to-age conversion with ed25519 keys, and skip behavior for RSA keys

## 5. Filesystem operations

- [x] 5.1 Implement ramfs/tmpfs mount at `secretsMountPoint` using `nix::mount::mount()`
- [x] 5.2 Implement generation directory creation: `<mountpoint>/<generation>/` with mode 0751
- [x] 5.3 Implement secret file writing with mode/owner/group from manifest (resolve names via `nix::unistd`)
- [x] 5.4 Implement atomic symlink update: create in temp dir, chown, rename to `symlinkPath`
- [x] 5.5 Implement per-secret symlinks at each secret's `path` field
- [x] 5.6 Implement generation pruning: remove directories older than `keepGenerations`

## 6. Template rendering and service tracking

- [x] 6.1 Implement template rendering: replace placeholders from `placeholderBySecretName` with decrypted values
- [x] 6.2 Implement template source loading: `content` field or `file` field
- [x] 6.3 Write rendered templates to `<generation>/rendered/<name>`
- [x] 6.4 Implement change detection: compare current vs previous generation files
- [x] 6.5 Write changed units to `/run/nixos/activation-restart-list` and `activation-reload-list`
- [x] 6.6 Handle dry activation: use `/run/nixos/dry-activation-*` prefix when `NIXOS_ACTION=dry-activate`

## 7. Check modes

- [x] 7.1 Implement `manifest` check mode: parse + validate manifest structure, exit without installing
- [x] 7.2 Implement `sopsfile` check mode: verify SOPS files exist, parse them, validate key paths exist in encrypted data
- [x] 7.3 Wire check modes into main flow (early exit before mount/decrypt)

## 8. NixOS integration

- [x] 8.1 Add `aspen-sops-install-secrets` package to `flake.nix` (unit2nix build)
- [x] 8.2 Create NixOS module `nix/modules/aspen-sops.nix`: override `sops.package`, add `sops.aspenTransit.clusterTicket` option, wire env var
- [x] 8.3 Ensure binary is named `sops-install-secrets` in the package output (rename or symlink)

## 9. Integration test

- [x] 9.1 Create `nix/tests/sops-install-secrets.nix`: NixOS VM test with sops-nix + aspen-sops-install-secrets module
- [x] 9.2 Test age-only lifecycle: generate age key, encrypt with `aspen-sops --age`, boot, verify secret at `/run/secrets/<name>`
- [x] 9.3 Test Transit + age lifecycle: start Aspen node, encrypt with Transit + age, boot with module, verify Transit-based decrypt
- [x] 9.4 Test template rendering: define a template with placeholder, verify rendered output
- [x] 9.5 Test permissions: verify secret file mode/owner/group match manifest
