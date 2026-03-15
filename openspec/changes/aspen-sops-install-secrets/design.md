## Context

sops-nix decrypts secrets at NixOS activation time using a Go binary (`sops-install-secrets`) that reads a manifest JSON describing secrets, their source SOPS files, and where to install them. The manifest format is stable and well-defined — Nix generates it at build time, the binary consumes it at boot.

The Go binary does three things:

1. **Decrypt**: Calls `github.com/getsops/sops/v3/decrypt.File()` for each SOPS file
2. **Install**: Writes decrypted values to a ramfs with correct permissions/ownership
3. **Manage**: Symlinks, generation pruning, template rendering, unit restart tracking

Aspen already has a complete SOPS decrypt implementation in `aspen_secrets::sops::decrypt`. The file management logic is mechanical Go that translates directly to Rust.

## Goals / Non-Goals

**Goals:**

- Drop-in replacement for `sops-install-secrets` that reads the same manifest JSON
- Decrypt SOPS files using `aspen_secrets::sops` (Transit + age backends)
- Full feature parity: ramfs mount, permission management, symlinks, generation pruning, templates, dry-activation support, SSH-to-age key conversion
- NixOS module overlay for easy integration: `sops.package = aspen-sops-install-secrets;`
- Integration test proving encrypt → boot → decrypt lifecycle

**Non-Goals:**

- GPG/PGP support (age + Transit covers Aspen's use cases)
- dotenv/ini format support in initial version (yaml/json/binary cover sops-nix defaults)
- User-mode (non-root) sops-nix operation
- Replacing the Nix-side manifest generation (manifest-for.nix stays as-is)

## Decisions

### 1. Single binary, no library crate

The `sops-install-secrets` replacement is a binary-only crate. There's no reuse case for the manifest parsing or file management logic outside this binary. All SOPS crypto lives in `aspen-secrets` already.

Alternative: Split into lib + bin. Rejected — adds crate overhead for code that's only used in one place.

### 2. Parse same manifest JSON schema

The manifest schema is defined implicitly by the Go struct tags in `main.go`. We define equivalent Rust structs with serde derives. No schema changes — the Nix-side `manifest-for.nix` stays untouched.

Key manifest fields:

- `secrets[]`: name, key, path, sopsFile, format, mode, owner/uid, group/gid, restartUnits, reloadUnits
- `templates[]`: name, content, file, path, mode, owner/uid, group/gid, restartUnits, reloadUnits
- `placeholderBySecretName`: map for template rendering
- `secretsMountPoint`, `symlinkPath`, `keepGenerations`
- `sshKeyPaths`, `gnupgHome`, `ageKeyFile`, `ageSshKeyPaths`
- `useTmpfs`, `userMode`, `logging`

### 3. Decrypt via aspen_secrets with format-aware key extraction

The Go code calls `decrypt.File(sopsFile, format)` which returns the full decrypted document, then extracts the requested key path from the parsed result. We do the same:

1. Call `aspen_secrets::sops::decrypt::decrypt_file()` to get the full decrypted document
2. Parse the decrypted output (yaml/json) and extract the key path
3. For binary format, use the raw bytes

The decrypt config needs a cluster ticket (from env var `ASPEN_CLUSTER_TICKET`) or falls back to age key file from the manifest's `ageKeyFile` / `ageSshKeyPaths`.

### 4. SSH-to-age key conversion via the `ssh-to-age` crate

The Go binary uses `github.com/Mic92/ssh-to-age` to convert SSH ed25519 keys to age identities. We use the Rust `ssh-to-age` crate (or implement the conversion — it's ed25519 key material repackaging). This is needed because sops-nix commonly configures `age.sshKeyPaths` pointing to the host's SSH keys.

Alternative: Require explicit age key files, skip SSH conversion. Rejected — breaks compatibility with standard sops-nix configurations.

### 5. Ramfs/tmpfs management via direct syscalls

Mount a ramfs (or tmpfs if configured) at `secretsMountPoint` using `nix` crate's `mount()`. The Go code does `syscall.Mount("ramfs", ...)` — direct translation. No external mount binary needed.

### 6. Remove keyservice bridge from aspen-sops

With a native Rust `sops-install-secrets`, the gRPC keyservice bridge is unnecessary. Remove:

- `crates/aspen-sops/src/keyservice/` directory
- `crates/aspen-sops/proto/keyservice.proto`
- `keyservice` feature flag from `Cargo.toml`
- tonic/tonic-build/prost dependencies
- Go SOPS interop subtests from `sops-transit.nix`
- `pkgs.sops` from test node packages

Keep `HcVaultTransitRecipient`, `SopsKeyGroup`, and `sync_key_groups()` in metadata — they're cheap and might be useful later.

### 7. Check modes for Nix build-time validation

The Go binary supports `-check-mode=manifest` (validate JSON structure) and `-check-mode=sopsfile` (validate SOPS files exist and contain expected keys). sops-nix uses this during `nix build` to catch errors early. We implement the same flags.

## Risks / Trade-offs

- **[Risk] SSH-to-age conversion edge cases** → Use the same algorithm as the Go library. Test with ed25519 keys. RSA SSH keys are GPG-only and out of scope.
- **[Risk] Manifest schema drift** → The manifest format is stable (years of sops-nix usage). Pin to current schema, add unknown-field tolerance with `#[serde(deny_unknown_fields)]` off.
- **[Risk] Missing format support** → Initial version skips dotenv/ini. If a user configures those formats, fail with a clear error message pointing them to yaml/json.
- **[Trade-off] No GPG support** → Aspen targets age + Transit. GPG users keep using stock sops-nix. Document this limitation.
