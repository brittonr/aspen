#!/usr/bin/env bash
# Helpers for preparing the run-forge-web demo repository.
# The caller must define an `err` logging function before sourcing this file.

set -euo pipefail

generate_demo_cargo_lockfile() (
  unset CARGO_INCREMENTAL
  RUSTC_WRAPPER='' cargo generate-lockfile -q
  test -s Cargo.lock
)

generate_demo_flake_lockfile() {
  if ! command -v nix >/dev/null 2>&1; then
    err "nix CLI not found; cannot generate flake.lock for the CI demo"
    return 1
  fi

  nix --extra-experimental-features "nix-command flakes" flake lock --quiet
  test -s flake.lock
}

require_staged_file() {
  local file_path="$1"

  if ! git ls-files --error-unmatch "$file_path" >/dev/null 2>&1; then
    err "$file_path was not staged; refusing to push a demo that CI cannot build reproducibly"
    exit 1
  fi
}

write_demo_readme() {
  cat > README.md << 'INNER_MD'
# Hello World

A demo repository hosted on **Aspen Forge**.

## Features

- Decentralized hosting — no single point of failure
- BLAKE3 content-addressed objects
- Raft-consistent refs
- P2P replication via iroh QUIC
- Web UI served over HTTP/3
- Nostr NIP-07 login (browser extension)
- Code search across repos
- Issue tracking with close/reopen
- Patch submission, review, and merge
- CI auto-trigger via `.aspen/ci.ncl`
INNER_MD
}

write_demo_cargo_project() {
  mkdir -p src
  printf 'fn main() {\n    println!("Hello from Aspen Forge!");\n}\n' > src/main.rs
  printf '[package]\nname = "hello"\nversion = "0.1.0"\nedition = "2024"\n' > Cargo.toml
}

write_demo_flake() {
  cat > flake.nix << 'INNER_FLAKE'
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "hello";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
      };
    };
}
INNER_FLAKE
}

write_demo_ci_config() {
  mkdir -p .aspen
  cat > .aspen/ci.ncl << 'INNER_NCL'
{
  name = "hello-world",
  stages = [
    {
      name = "build",
      jobs = [
        {
          name = "nix-build",
          type = 'nix,
          flake_attr = "packages.x86_64-linux.default",
          timeout_secs = 600,
        },
      ],
    },
  ],
}
INNER_NCL
}

commit_forge_web_demo_repo() {
  write_demo_readme
  write_demo_cargo_project

  # Generate Cargo.lock so the Nix build can vendor dependencies.
  # The temp repo lives outside Aspen's .cargo/config.toml, so clear host
  # wrappers that reject the dev shell's CARGO_INCREMENTAL setting.
  if ! generate_demo_cargo_lockfile; then
    err "Failed to generate Cargo.lock for the demo repository"
    exit 1
  fi

  write_demo_flake

  # Nix flakes loaded from a Git working tree only see tracked files, so
  # stage flake.nix before asking Nix to create the lock file.
  if ! git add flake.nix; then
    err "Failed to stage flake.nix before generating flake.lock"
    exit 1
  fi

  # Commit flake.lock so CI uses the same pinned nixpkgs input instead of
  # creating a lock file inside the checkout at build time.
  if ! generate_demo_flake_lockfile; then
    err "Failed to generate flake.lock for the demo repository"
    exit 1
  fi

  write_demo_ci_config

  if ! git add -A; then
    err "Failed to stage demo repository"
    exit 1
  fi
  require_staged_file Cargo.lock
  require_staged_file flake.lock
  if ! git commit -q -m "Initial commit"; then
    err "Failed to commit demo repository"
    exit 1
  fi
}
