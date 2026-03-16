## Why

sops-nix is the standard NixOS module for managing encrypted secrets. It uses a Go binary (`sops-install-secrets`) that links the Go SOPS library to decrypt secrets at boot. To use Aspen Transit as a sops-nix backend, we currently need Go SOPS interop — a gRPC keyservice bridge that doesn't work and adds protobuf build complexity. Writing a Rust drop-in replacement for `sops-install-secrets` eliminates the Go dependency entirely and lets sops-nix decrypt Aspen Transit-encrypted files natively.

## What Changes

- New `aspen-sops-install-secrets` binary crate that reads the sops-nix manifest JSON and decrypts secrets using `aspen_secrets::sops`
- Handles the full sops-nix lifecycle: ramfs mount, secret decryption, file permissions, symlink management, generation pruning, template rendering
- Drop-in replacement via `sops.package = aspen-sops-install-secrets;` in NixOS config
- Supports age key files for boot-time decryption (no cluster needed) and Aspen Transit when cluster is reachable
- Remove the gRPC keyservice bridge (`keyservice` feature) from `aspen-sops` — no longer needed
- Remove Go SOPS interop test subtests from `sops-transit.nix`
- Remove `pkgs.sops` dependency from NixOS test nodes

## Capabilities

### New Capabilities

- `sops-install-secrets`: Rust replacement for Go `sops-install-secrets` binary — manifest parsing, secret decryption, file management, template rendering, generation pruning
- `sops-nixos-module`: NixOS module overlay that wires `aspen-sops-install-secrets` as the sops-nix package and adds Aspen Transit key configuration options

### Modified Capabilities
<!-- No existing spec-level requirements change -->

## Impact

- **New crate**: `crates/aspen-sops-install-secrets/` (binary crate, depends on `aspen-secrets`)
- **Modified crate**: `crates/aspen-sops/` — remove `keyservice` feature, proto file, tonic dependency
- **Nix**: New package in `flake.nix`, new NixOS module in `nix/modules/`, updated sops-transit test
- **Build**: Removes protobuf/tonic codegen from `aspen-sops`, simplifying unit2nix build
- **Dependencies**: No new external deps — `aspen-secrets` already handles all SOPS decryption
