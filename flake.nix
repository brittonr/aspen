{
  description = "molten — Rust project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    unit2nix = {
      url = "github:brittonr/unit2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    onix-kache-lib = {
      url = "path:/home/brittonr/git/onix-core/lib";
      flake = false;
    };
    onix-kache-package-src = {
      url = "path:/home/brittonr/git/onix-core/pkgs/kache";
      flake = false;
    };
    basalt-src = {
      url = "path:/home/brittonr/.cargo/git/checkouts/basalt-d217f0a83bebd193/d913dc0";
      flake = false;
    };
    cairn-src = {
      url = "path:/home/brittonr/.cargo/git/checkouts/cairn-d7a4d31a0615cac1/3b4c280";
      flake = false;
    };
    octet-src = {
      url = "path:/home/brittonr/.cargo/git/checkouts/octet-d771f362f4abe884/9b6a206";
      flake = false;
    };
    ucan-src = {
      url = "path:/home/brittonr/.cargo/git/checkouts/ucan-9abe9593165792e6/2aad993";
      flake = false;
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      unit2nix,
      rust-overlay,
      flake-utils,
      onix-kache-lib,
      onix-kache-package-src,
      basalt-src,
      cairn-src,
      octet-src,
      ucan-src,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgsBase = import nixpkgs {
          localSystem = system;
          overlays = [ (import rust-overlay) ];
        };

        localGitSources = {
          "ssh://git@github.com/OnixResearch/basalt.git#d913dc01e765c9b297df5fcc57dfa06aac39bc74" =
            basalt-src;
          "ssh://git@github.com/OnixResearch/cairn.git#3b4c280b893f2709aebea21fc51a4f9eeba3fe3b" = cairn-src;
          "ssh://git@github.com/OnixResearch/octet.git#9b6a2065ef9e8e363d81299cf59d74f885926215" = octet-src;
          "ssh://git@github.com/OnixResearch/ucan.git#2aad993027d48ff148028c537cdaf91f6e5285ca" = ucan-src;
        };

        pkgs = pkgsBase;
        unit2nixPkgsBase = pkgsBase.extend (
          final: prev: {
            fetchgit =
              (prev.lib.makeOverridable (
                args:
                let
                  localGitKey = if args ? rev then "${args.url}#${args.rev}" else "";
                  localGitSource = localGitSources.${localGitKey} or null;
                in
                if localGitSource != null then
                  prev.runCommand (args.name or "local-git-source") { } ''
                    cp -R ${localGitSource} "$out"
                    chmod -R u+w "$out"
                    ${args.postFetch or ""}
                  ''
                else
                  prev.fetchgit args
              ))
              // {
                inherit (prev.fetchgit) getRevWithTag;
              };
          }
        );

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rustToolchainCompat = rustToolchain // {
          unwrapped = rustToolchain // {
            configureFlags = [ "--target=${pkgs.stdenv.hostPlatform.rust.rustcTarget}" ];
          };
        };
        unit2nixPkgs = unit2nixPkgsBase.extend (
          final: prev: {
            # unit2nix auto mode forwards the custom toolchain to Cargo's
            # unit-graph generation, but this pinned revision does not forward it
            # to the clippy wrapper. Make pkgs.clippy/pkgs.rustc match the
            # dependency compiler so Nix flake checks do not mix rustc metadata.
            clippy = rustToolchain;
            rustc = rustToolchainCompat;
          }
        );

        kacheCacheDir = "/var/cache/kache-nix";
        kacheKeySalt = "molten-unit2nix-kache-v1";
        kachePackage = pkgs.callPackage onix-kache-package-src { };
        kacheLib = import (onix-kache-lib + "/kache-nix-rust.nix") {
          lib = pkgs.lib;
          inherit pkgs kachePackage;
        };
        mkUnit2nixRust =
          {
            enableKache ? false,
            cacheDir ? kacheCacheDir,
            keySalt ? kacheKeySalt,
          }:
          if enableKache then
            kacheLib.mkWrappedRustPackage {
              name = "molten-kache-rust";
              rust = rustToolchain;
              inherit cacheDir keySalt;
            }
          else
            rustToolchain;
        mkBuildRustCrateForPkgs =
          {
            enableKache ? false,
            cacheDir ? kacheCacheDir,
            keySalt ? kacheKeySalt,
          }:
          pkgs:
          let
            unit2nixRust = mkUnit2nixRust { inherit enableKache cacheDir keySalt; };
          in
          pkgs.buildRustCrate.override {
            cargo = unit2nixRust;
            rustc = unit2nixRust;
          };
        mkUnit2nixWorkspace =
          {
            enableKache ? false,
            cacheDir ? kacheCacheDir,
            keySalt ? kacheKeySalt,
          }:
          unit2nix.lib.${system}.buildFromUnitGraph {
            pkgs = unit2nixPkgs;
            inherit rustToolchain;
            src = ./.;
            # Keep the unit graph checked in so package evaluation does not
            # depend on unit2nix IFD.
            resolvedJson = ./build-plan.json;
            clippyArgs = [
              "-D"
              "warnings"
            ];
            buildRustCrateForPkgs = mkBuildRustCrateForPkgs { inherit enableKache cacheDir keySalt; };
            extraCrateOverrides = {
              # nickel-lang-core declares links="nix", but the Nix FFI is behind
              # the disabled nix-experimental feature in this workspace.
              nickel-lang-core = attrs: { };
              # verus_prettyplease declares links="prettyplease-verus02" but
              # vendors its implementation; no native libraries are required.
              verus_prettyplease = attrs: { };
            };
          };

        ws = mkUnit2nixWorkspace { enableKache = false; };
        kacheWs = mkUnit2nixWorkspace { enableKache = true; };
        kacheWrappedRust = mkUnit2nixRust { enableKache = true; };

        moltenPkg = ws.workspaceMembers."molten".build;
        moltenTestBinaries = (ws.test.workspaceMembers."molten".build).override { buildTests = true; };
        targetTriple = pkgs.stdenv.hostPlatform.rust.rustcTarget;
        rustLibDir = "${rustToolchain}/lib/rustlib/${targetTriple}/lib";
        nextestCi = pkgs.writeShellApplication {
          name = "molten-nextest-ci";
          runtimeInputs = [
            rustToolchain
            pkgs.cargo-nextest
          ];
          text = ''
            exec cargo nextest run --profile ci "$@"
          '';
        };
        sourceForConfigChecks = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter =
            path: type:
            let
              base = baseNameOf path;
            in
            !(base == "target" || base == ".direnv" || base == ".git");
        };

        moltenVmNodeModule =
          nodeId:
          { pkgs, ... }:
          {
            virtualisation.graphics = false;
            networking.hostName = nodeId;
            networking.firewall.enable = false;
            networking.extraHosts = ''
              192.168.1.1 node-a node_a
              192.168.1.2 node-b node_b
            '';
            environment.systemPackages = [
              moltenPkg
              pkgs.coreutils
              pkgs.gnugrep
              pkgs.iputils
            ];
            systemd.services.molten-node = {
              description = "Molten node VM integration service";
              wantedBy = [ "multi-user.target" ];
              after = [ "network-online.target" ];
              wants = [ "network-online.target" ];
              path = [
                moltenPkg
                pkgs.coreutils
                pkgs.gnugrep
              ];
              serviceConfig = {
                Type = "oneshot";
                RemainAfterExit = true;
                StateDirectory = "molten";
                WorkingDirectory = "${sourceForConfigChecks}";
                ExecStop = "${moltenPkg}/bin/molten node stop --state-root /var/lib/molten --shutdown-out /var/lib/molten/vm-evidence/shutdown.preserves --receipt-out /var/lib/molten/vm-evidence/shutdown-control.preserves";
              };
              script = ''
                set -euo pipefail
                state=/var/lib/molten
                evidence="$state/vm-evidence"
                mkdir -p "$evidence"
                if [ ! -f "$state/config.preserves" ]; then
                  molten node init \
                    --state-root "$state" \
                    --node-id "node:${nodeId}" \
                    --config-out "$evidence/node-config.preserves" \
                    --identity-receipt-out "$evidence/identity.preserves" \
                    > "$evidence/init.txt"
                fi
                molten node run \
                  --state-root "$state" \
                  --startup-out "$evidence/startup.preserves" \
                  > "$evidence/run.txt"
                molten node status \
                  --state-root "$state" \
                  --health-out "$evidence/health.preserves" \
                  --receipt-out "$evidence/status.preserves" \
                  > "$evidence/status.txt"
                molten node run-loop \
                  --state-root "$state" \
                  --max-requests 1 \
                  --receipt-out "$evidence/control-loop.preserves" \
                  --heartbeat-out "$evidence/heartbeat.preserves" \
                  > "$evidence/run-loop.txt"
              '';
            };
            system.stateVersion = "24.11";
          };
      in
      {
        packages = {
          default = moltenPkg;
          molten = moltenPkg;
          molten-kache = kacheWs.workspaceMembers."molten".build;
          molten-kache-rust = kacheWrappedRust;
          all = ws.allWorkspaceMembers;
        };

        checks = rec {
          # The hermetic nextest check supplies binary metadata for CLI tests
          # using CARGO_BIN_EXE_molten; the raw unit2nix libtest runner does not.
          molten = nextest;
          clippy = ws.clippy.allWorkspaceMembers;

          deterministic-drift-gate =
            pkgs.runCommand "molten-deterministic-drift-gate"
              {
                nativeBuildInputs = [ moltenPkg ];
                src = sourceForConfigChecks;
              }
              ''
                set -euo pipefail
                mkdir -p "$out"
                cp -R $src source
                chmod -R u+w source
                cd source

                printf '%s\n' '<release-evidence-fixture "stable">' > left.preserves
                cp left.preserves right.preserves
                molten test drift compare \
                  --workflow release-evidence \
                  --left left.preserves \
                  --right right.preserves \
                  --out "$out/drift-pass.preserves" \
                  > "$out/drift-pass.txt"

                molten test drift rerun \
                  --workflow nixos-vm-topology \
                  --left-root "$TMPDIR/left-root" \
                  --right-root "$TMPDIR/right-root" \
                  --command molten \
                  --arg test \
                  --arg nixos-vm \
                  --arg topology \
                  --arg=--node \
                  --arg node_a \
                  --arg=--node \
                  --arg node_b \
                  --arg=--package-ref \
                  --arg blake3:0000000000000000000000000000000000000000000000000000000000000000 \
                  --arg=--package-path \
                  --arg /nix/store/example-molten \
                  --arg=--network \
                  --arg nixos-test-private \
                  --arg=--caveat \
                  --arg 'topology rerun fixture is deterministic evidence only' \
                  --arg=--out \
                  --arg '{root}/topology.preserves' \
                  --artifact topology.preserves \
                  --out "$out/topology-drift-rerun.preserves" \
                  > "$out/topology-drift-rerun.txt"

                printf '%s\n' '<release-evidence-fixture "drifted">' > drifted.preserves
                if molten test drift compare \
                  --workflow release-evidence \
                  --left left.preserves \
                  --right drifted.preserves \
                  --out "$out/drift-deny.preserves" \
                  > "$out/drift-deny.txt" 2> "$out/drift-deny.err"; then
                  echo "negative drift fixture unexpectedly passed" >&2
                  exit 1
                fi
              '';

          requirement-traceability-gate =
            pkgs.runCommand "molten-requirement-traceability-gate"
              {
                nativeBuildInputs = [ moltenPkg ];
              }
              ''
                set -euo pipefail
                mkdir -p "$out" fixture/cairn/changes/traceability/specs/testing-harness fixture/tests
                {
                  printf '%s\n' '## ADDED Requirements'
                  printf '\n'
                  printf '%s\n' '### Requirement: Traceability fixture'
                  printf '%s\n' 'r[molten.testing.traceability.fixture] Molten MUST bind positive and negative coverage in this fixture.'
                } > fixture/cairn/changes/traceability/specs/testing-harness/spec.md
                touch fixture/tests/coverage.rs
                artifact_ref=blake3:0000000000000000000000000000000000000000000000000000000000000000
                positive='molten.testing.traceability.fixture|positive|tests/coverage.rs|cargo test coverage|'
                negative='molten.testing.traceability.fixture|negative|tests/coverage.rs|cargo test coverage|'
                molten test traceability scan \
                  --root fixture \
                  --changed-only \
                  --coverage "$positive$artifact_ref" \
                  --coverage "$negative$artifact_ref" \
                  --out "$out/traceability-pass.preserves" \
                  --summary-out "$out/traceability-pass.txt"

                if molten test traceability scan \
                  --root fixture \
                  --changed-only \
                  --coverage "$positive$artifact_ref" \
                  --out "$out/traceability-deny.preserves" \
                  --summary-out "$out/traceability-deny.txt" \
                  > "$out/traceability-deny.stdout" 2> "$out/traceability-deny.stderr"; then
                  echo "negative traceability fixture unexpectedly passed" >&2
                  exit 1
                fi
              '';

          production-profile-fixtures =
            pkgs.runCommand "molten-production-profile-fixtures"
              {
                nativeBuildInputs = [
                  pkgs.nickel
                  pkgs.diffutils
                ];
                src = sourceForConfigChecks;
              }
              ''
                set -euo pipefail
                cp -R $src source
                chmod -R u+w source
                cd source

                nickel export docs/production-node-profile.ncl > "$TMPDIR/production-node-profile.json"
                nickel export docs/production-profile-fixtures/valid.ncl > "$TMPDIR/production-profile-valid.json"
                nickel export docs/production-node-profile.ncl --field profile.resource_limits > "$TMPDIR/resource-limits.json"
                diff -u docs/production-profile-fixtures/expected-resource-limits.json "$TMPDIR/resource-limits.json"

                failed=0
                for fixture in docs/production-profile-fixtures/negative/*.ncl; do
                  name=$(basename "$fixture")
                  if nickel export "$fixture" > "$TMPDIR/$name.json" 2> "$TMPDIR/$name.err"; then
                    echo "negative fixture unexpectedly exported: $fixture" >&2
                    failed=1
                  fi
                done
                if [ "$failed" -ne 0 ]; then
                  exit 1
                fi
                touch "$out"
              '';

          contract-export-drift-gate =
            pkgs.runCommand "molten-contract-export-drift-gate"
              {
                nativeBuildInputs = [
                  pkgs.nickel
                  pkgs.diffutils
                ];
                src = sourceForConfigChecks;
              }
              ''
                set -euo pipefail
                cp -R $src source
                chmod -R u+w source
                cd source

                nickel export cairn-policy/default.ncl > "$TMPDIR/cairn-policy.json"
                diff -u cairn-policy/generated/cairn-policy.json "$TMPDIR/cairn-policy.json"

                nickel export docs/plugin-extension-contracts/storage.contract-envelope.ncl > "$TMPDIR/storage.contract-envelope.json"
                diff -u docs/plugin-extension-contracts/generated/storage.contract-envelope.json "$TMPDIR/storage.contract-envelope.json"
                nickel export docs/plugin-extension-contracts/storage.grant-envelope.ncl > "$TMPDIR/storage.grant-envelope.json"
                diff -u docs/plugin-extension-contracts/generated/storage.grant-envelope.json "$TMPDIR/storage.grant-envelope.json"

                nickel export docs/production-node-profile.ncl --field profile.resource_limits > "$TMPDIR/resource-limits.json"
                diff -u docs/production-profile-fixtures/expected-resource-limits.json "$TMPDIR/resource-limits.json"

                failed=0
                positive_fixture() {
                  label="$1"
                  fixture="$2"
                  if ! nickel export "$fixture" > "$TMPDIR/$label.json" 2> "$TMPDIR/$label.err"; then
                    echo "positive fixture failed: $fixture" >&2
                    cat "$TMPDIR/$label.err" >&2
                    failed=1
                  fi
                }
                negative_fixture() {
                  label="$1"
                  fixture="$2"
                  if nickel export "$fixture" > "$TMPDIR/$label.json" 2> "$TMPDIR/$label.err"; then
                    echo "negative fixture unexpectedly exported: $fixture" >&2
                    failed=1
                  fi
                }

                positive_fixture production-node-profile docs/production-node-profile.ncl
                positive_fixture production-profile-valid docs/production-profile-fixtures/valid.ncl
                for fixture in docs/production-profile-fixtures/negative/*.ncl; do
                  negative_fixture "production-$(basename "$fixture" .ncl)" "$fixture"
                done
                positive_fixture peer-profile-valid docs/peer-profile-fixtures/valid.ncl
                for fixture in docs/peer-profile-fixtures/negative/*.ncl; do
                  negative_fixture "peer-$(basename "$fixture" .ncl)" "$fixture"
                done
                for fixture in docs/multinode-scenario-fixtures/valid/*.ncl; do
                  positive_fixture "multinode-valid-$(basename "$fixture" .ncl)" "$fixture"
                done
                for fixture in docs/multinode-scenario-fixtures/negative/*.ncl; do
                  negative_fixture "multinode-negative-$(basename "$fixture" .ncl)" "$fixture"
                done
                positive_fixture plugin-storage docs/plugin-extension-contracts/storage.ncl
                positive_fixture plugin-storage-grant docs/plugin-extension-contracts/storage.grant.ncl
                positive_fixture plugin-storage-revoked-grant docs/plugin-extension-contracts/storage-revoked.grant.ncl
                positive_fixture plugin-storage-contract-envelope docs/plugin-extension-contracts/storage.contract-envelope.ncl
                positive_fixture plugin-storage-grant-envelope docs/plugin-extension-contracts/storage.grant-envelope.ncl
                for fixture in docs/plugin-extension-contracts/storage-*.ncl docs/plugin-extension-contracts/storage-*.grant.ncl; do
                  case "$fixture" in
                    docs/plugin-extension-contracts/storage-revoked.grant.ncl|docs/plugin-extension-contracts/storage.contract-envelope.ncl|docs/plugin-extension-contracts/storage.grant-envelope.ncl) continue ;;
                  esac
                  negative_fixture "plugin-$(basename "$fixture" .ncl)" "$fixture"
                done
                positive_fixture cairn-policy-default cairn-policy/default.ncl
                for fixture in cairn-policy/fixtures/*.ncl; do
                  name=$(basename "$fixture" .ncl)
                  case "$name" in
                    valid*) positive_fixture "cairn-$name" "$fixture" ;;
                    *) negative_fixture "cairn-$name" "$fixture" ;;
                  esac
                done

                if [ "$failed" -ne 0 ]; then
                  exit 1
                fi
                touch "$out"
              '';

          kache-nix-rust-wrapper-contract =
            let
              missingCacheDiagnostic = "cache directory is not writable";
              fakeKache = pkgs.writeShellApplication {
                name = "kache";
                text = ''
                  if [ -n "''${KACHE_NIX_TRACE:-}" ]; then
                    {
                      printf 'fake_kache_invoked=true\n'
                      printf 'argv=%s\n' "$*"
                      printf 'KACHE_KEY_SALT=%s\n' "''${KACHE_KEY_SALT:-}"
                      printf 'KACHE_CACHE_DIR=%s\n' "''${KACHE_CACHE_DIR:-}"
                    } >> "$KACHE_NIX_TRACE"
                  fi
                  exec "$@"
                '';
              };
              checkedKacheLib = import (onix-kache-lib + "/kache-nix-rust.nix") {
                lib = pkgs.lib;
                inherit pkgs;
                kachePackage = fakeKache;
              };
              checkedWrappedRust = checkedKacheLib.mkWrappedRustPackage {
                name = "molten-checked-kache-rust";
                rust = rustToolchain;
                cacheDir = kacheCacheDir;
                keySalt = kacheKeySalt;
              };
              disabledRust = mkUnit2nixRust { enableKache = false; };
            in
            pkgs.runCommand "molten-kache-nix-rust-wrapper-contract" { } ''
              set -eu

              if [ ${pkgs.lib.escapeShellArg (toString disabledRust)} != ${pkgs.lib.escapeShellArg (toString rustToolchain)} ]; then
                echo "negative: disabled unit2nix path must use the unwrapped Rust toolchain" >&2
                exit 1
              fi
              if [ ${pkgs.lib.escapeShellArg (toString checkedWrappedRust)} = ${pkgs.lib.escapeShellArg (toString rustToolchain)} ]; then
                echo "positive: enabled unit2nix path must use a wrapped Rust toolchain" >&2
                exit 1
              fi

              wrapper=${checkedWrappedRust}/bin/rustc
              cache_dir="$PWD/cache"
              trace="$PWD/kache.trace"

              KACHE_NIX_DISABLED=1 KACHE_NIX_TRACE="$PWD/disabled.trace" "$wrapper" -vV > "$PWD/disabled-rustc-version.txt"
              if [ -s "$PWD/disabled.trace" ]; then
                echo "negative: disabled mode should not invoke kache" >&2
                exit 1
              fi

              if KACHE_NIX_CACHE_DIR="$PWD/missing-cache" "$wrapper" -vV > "$PWD/missing-cache.stdout" 2> "$PWD/missing-cache.stderr"; then
                echo "negative: missing cache directory unexpectedly succeeded" >&2
                exit 1
              fi
              if ! ${pkgs.gnugrep}/bin/grep -Fq ${pkgs.lib.escapeShellArg missingCacheDiagnostic} "$PWD/missing-cache.stderr"; then
                echo "negative: missing cache directory did not produce the expected diagnostic" >&2
                cat "$PWD/missing-cache.stderr" >&2
                exit 1
              fi

              mkdir -p "$cache_dir"
              KACHE_NIX_CACHE_DIR="$cache_dir" KACHE_NIX_TRACE="$trace" "$wrapper" -vV > "$PWD/wrapped-rustc-version.txt"
              if ! ${pkgs.gnugrep}/bin/grep -Fq 'fake_kache_invoked=true' "$trace"; then
                echo "positive: wrapper did not invoke kache" >&2
                cat "$trace" >&2
                exit 1
              fi
              if ! ${pkgs.gnugrep}/bin/grep -Fq "KACHE_CACHE_DIR=$cache_dir" "$trace"; then
                echo "positive: wrapper did not export the runtime cache directory" >&2
                cat "$trace" >&2
                exit 1
              fi
              if ! ${pkgs.gnugrep}/bin/grep -Fq 'operator=${kacheKeySalt}' "$trace"; then
                echo "positive: wrapper did not include the operator key salt" >&2
                cat "$trace" >&2
                exit 1
              fi
              if ! [ -x ${checkedWrappedRust}/bin/rustdoc ]; then
                echo "positive: wrapped rust package must preserve rustdoc compatibility" >&2
                exit 1
              fi
              if ${pkgs.gnugrep}/bin/grep -R -Fq '/home/brittonr/.cache/kache' ${checkedWrappedRust}; then
                echo "negative: wrapper must not reference the user-level kache cache" >&2
                exit 1
              fi
              if ${pkgs.gnugrep}/bin/grep -R -Fq '/home/brittonr/.cargo/config.toml' ${checkedWrappedRust}; then
                echo "negative: wrapper must not read user-level Cargo config" >&2
                exit 1
              fi

              touch "$out"
            '';

          nextest =
            pkgs.runCommand "molten-nextest"
              {
                nativeBuildInputs = [
                  rustToolchain
                  pkgs.cargo-nextest
                  pkgs.perl
                  moltenPkg
                ];
                src = sourceForConfigChecks;
                testBinaries = moltenTestBinaries;
                inherit targetTriple rustLibDir;
              }
              ''
                set -euo pipefail
                export HOME="$TMPDIR/home"
                mkdir -p "$HOME"
                cp -R $src source
                chmod -R u+w source
                cd source
                cargo metadata --format-version 1 --no-deps --locked > cargo-metadata.json
                perl -MJSON::PP -e '
                  use strict;
                  use warnings;
                  my $metadata_path = "cargo-metadata.json";
                  my $test_dir = "$ENV{testBinaries}/tests";
                  my $target_triple = $ENV{targetTriple};
                  my $rust_lib_dir = $ENV{rustLibDir};
                  die "missing unit2nix test binary directory: $test_dir\n" unless -d $test_dir;

                  my $metadata = decode_json(do { local $/; open my $fh, "<", $metadata_path or die "open $metadata_path: $!"; <$fh> });
                  my ($package) = grep { $_->{name} eq "molten" } @{$metadata->{packages}};
                  die "cargo metadata did not contain package molten\n" unless $package;

                  my @binaries;
                  for my $path (sort glob("$test_dir/*")) {
                    next unless -f $path && -x $path;
                    next unless system("$path --list --format terse >/dev/null 2>&1") == 0;
                    push @binaries, $path;
                  }
                  die "no libtest-compatible binaries found in $test_dir\n" unless @binaries;

                  my %rust_binaries;
                  my $index = 0;
                  for my $path (@binaries) {
                    (my $name = $path) =~ s{.*/}{};
                    my $id = "molten::nix-test/$index/$name";
                    $rust_binaries{$id} = {
                      "binary-id" => $id,
                      "binary-name" => $name,
                      "package-id" => $package->{id},
                      "kind" => "test",
                      "binary-path" => $path,
                      "build-platform" => "target",
                    };
                    $index++;
                  }

                  my $binaries_metadata = {
                    "rust-build-meta" => {
                      "target-directory" => $ENV{testBinaries},
                      "base-output-directories" => ["tests"],
                      "non-test-binaries" => {},
                      "build-script-out-dirs" => {},
                      "build-script-info" => {},
                      "linked-paths" => [],
                      "platforms" => {
                        "host" => {
                          "platform" => {
                            "triple" => $target_triple,
                            "target-features" => "unknown",
                          },
                          "libdir" => {
                            "status" => "available",
                            "path" => $rust_lib_dir,
                          },
                        },
                        "targets" => [],
                      },
                      "target-platforms" => [{
                        "triple" => $target_triple,
                        "target-features" => "unknown",
                      }],
                      "target-platform" => undef,
                    },
                    "rust-binaries" => \%rust_binaries,
                  };

                  open my $out, ">", "binaries-metadata.json" or die "write binaries-metadata.json: $!";
                  print $out JSON::PP->new->canonical->pretty->encode($binaries_metadata);
                '
                cargo nextest run \
                  --profile ci \
                  --user-config-file none \
                  --cargo-metadata cargo-metadata.json \
                  --binaries-metadata binaries-metadata.json \
                  --no-tests fail
                mkdir -p "$out"
                cp cargo-metadata.json binaries-metadata.json "$out"/
                if [ ! -s target/nextest/ci/junit.xml ]; then
                  echo "missing CI JUnit evidence at target/nextest/ci/junit.xml" >&2
                  find target/nextest -maxdepth 4 -type f | sort >&2 || true
                  exit 1
                fi
                cp target/nextest/ci/junit.xml "$out"/
                molten test traceability ci-run-receipt \
                  --source-marker "$src" \
                  --profile-id ci \
                  --command-surface 'cargo nextest run --profile ci --user-config-file none --cargo-metadata cargo-metadata.json --binaries-metadata binaries-metadata.json --no-tests fail' \
                  --nextest-config .config/nextest.toml \
                  --cargo-metadata cargo-metadata.json \
                  --binaries-metadata binaries-metadata.json \
                  --junit target/nextest/ci/junit.xml \
                  --caveat 'JUnit is a rendered view over canonical CI receipt metadata' \
                  --out "$out/ci-test-run-receipt.preserves"
              '';

          dogfood-local-node =
            pkgs.runCommand "molten-dogfood-local-node"
              {
                nativeBuildInputs = [
                  moltenPkg
                  pkgs.gnugrep
                ];
                src = sourceForConfigChecks;
                nextestCheck = nextest;
              }
              ''
                set -euo pipefail
                export HOME="$TMPDIR/home"
                mkdir -p "$HOME"
                cp -R $src source
                chmod -R u+w source
                cd source
                molten dogfood local-node \
                  --state-root "$TMPDIR/dogfood-state" \
                  --out dogfood-report.preserves \
                  --release-gate-out release-gate.preserves \
                  --replay-verify-out replay-verify.preserves \
                  --replay-index-out replay-evidence-index.preserves \
                  > dogfood-summary.txt
                grep -q 'decision=pass' dogfood-summary.txt
                grep -q 'dogfood-report-v1' dogfood-report.preserves
                grep -q 'release-gate-receipt-v1' release-gate.preserves
                grep -q 'deterministic-replay-verify-v1' replay-verify.preserves
                grep -q 'deterministic-replay-index-v1' replay-evidence-index.preserves
                mkdir -p "$out"
                cp dogfood-summary.txt dogfood-report.preserves release-gate.preserves replay-verify.preserves replay-evidence-index.preserves "$out"/
                printf '%s\n' "$nextestCheck" > "$out/after-nextest.txt"
                molten dogfood nix-release-export \
                  --output-path "$out" \
                  --out "$out/nix-dogfood-evidence.preserves"
                molten dogfood nix-release-verify \
                  --output-path "$out" \
                  --evidence "$out/nix-dogfood-evidence.preserves" \
                  --receipt-out "$out/nix-dogfood-verify.preserves" \
                  | tee "$out/nix-dogfood-verify.txt"
                grep -q 'decision=pass' "$out/nix-dogfood-verify.txt"
                molten dogfood release-bundle-export \
                  --output-path "$out" \
                  --out "$out/release-evidence-bundle.preserves"
                mkdir -p "$out/signed-keyring"
                molten receipts key import \
                  --ledger "$out/signed-keyring" \
                  --key-id local-release-key-v1 \
                  --signer local-release-signer \
                  --trust-root local-release-trust-root \
                  --key local-release-key \
                  > "$out/signed-keyring-import.txt"
                molten receipts sign "$out/dogfood-report.preserves" \
                  --out "$out/dogfood-report.signed.preserves" \
                  --signer local-release-signer \
                  --purpose release-evidence \
                  --trust-root local-release-trust-root \
                  --key local-release-key
                molten receipts sign "$out/release-gate.preserves" \
                  --out "$out/release-gate.signed.preserves" \
                  --signer local-release-signer \
                  --purpose release-evidence \
                  --trust-root local-release-trust-root \
                  --key local-release-key
                molten receipts sign "$out/replay-verify.preserves" \
                  --out "$out/replay-verify.signed.preserves" \
                  --signer local-release-signer \
                  --purpose release-evidence \
                  --trust-root local-release-trust-root \
                  --key local-release-key
                molten receipts sign "$out/replay-evidence-index.preserves" \
                  --out "$out/replay-evidence-index.signed.preserves" \
                  --signer local-release-signer \
                  --purpose release-evidence \
                  --trust-root local-release-trust-root \
                  --key local-release-key
                molten receipts sign "$out/nix-dogfood-evidence.preserves" \
                  --out "$out/nix-dogfood-evidence.signed.preserves" \
                  --signer local-release-signer \
                  --purpose release-evidence \
                  --trust-root local-release-trust-root \
                  --key local-release-key
                molten receipts sign "$out/nix-dogfood-verify.preserves" \
                  --out "$out/nix-dogfood-verify.signed.preserves" \
                  --signer local-release-signer \
                  --purpose release-evidence \
                  --trust-root local-release-trust-root \
                  --key local-release-key
                molten dogfood release-bundle-verify \
                  --output-path "$out" \
                  --bundle "$out/release-evidence-bundle.preserves" \
                  --receipt-out "$out/release-evidence-bundle-verify.preserves" \
                  --require-signed-members \
                  --signed-purpose release-evidence \
                  --signed-trust-root local-release-trust-root \
                  --signed-key-ledger "$out/signed-keyring" \
                  --signed-key-id local-release-key-v1 \
                  --signed-signer local-release-signer \
                  --signed-member "$out/dogfood-report.signed.preserves" \
                  --signed-member "$out/release-gate.signed.preserves" \
                  --signed-member "$out/replay-verify.signed.preserves" \
                  --signed-member "$out/replay-evidence-index.signed.preserves" \
                  --signed-member "$out/nix-dogfood-evidence.signed.preserves" \
                  --signed-member "$out/nix-dogfood-verify.signed.preserves" \
                  | tee "$out/release-evidence-bundle-verify.txt"
                grep -q 'decision=pass' "$out/release-evidence-bundle-verify.txt"
                molten dogfood release-promote \
                  --output-path "$out" \
                  --bundle-verify "$out/release-evidence-bundle-verify.preserves" \
                  --receipt-out "$out/release-promotion-gate.preserves" \
                  --signed-key-ledger "$out/signed-keyring" \
                  --signed-key-id local-release-key-v1 \
                  --signed-trust-root local-release-trust-root \
                  --signed-signer local-release-signer \
                  --source-evidence "flake-source:$src" \
                  --octet-evidence "octet:external-clean-gate-required" \
                  --cairn-evidence "cairn:external-strict-validate-required" \
                  | tee "$out/release-promotion-gate.txt"
                grep -q 'decision=pass' "$out/release-promotion-gate.txt"
                molten receipts sign "$out/release-promotion-gate.preserves" \
                  --out "$out/release-promotion-gate.signed.preserves" \
                  --signer local-release-signer \
                  --purpose release-promotion \
                  --trust-root local-release-trust-root \
                  --key local-release-key
                promotion_ref=$(sed -n 's/.* receipt=\([^ ]*\).*/\1/p' "$out/release-promotion-gate.txt")
                test -n "$promotion_ref"
                molten receipts verify-signed "$out/release-promotion-gate.signed.preserves" \
                  --purpose release-promotion \
                  --trust-root local-release-trust-root \
                  --key-ledger "$out/signed-keyring" \
                  --key-id local-release-key-v1 \
                  --signer local-release-signer \
                  --subject-ref "$promotion_ref" \
                  | tee "$out/release-promotion-gate-signed-verify.txt"
                grep -q 'receipts verify-signed ok' "$out/release-promotion-gate-signed-verify.txt"
                molten dogfood release-promotion-summary \
                  --output-path "$out" \
                  --out "$out/release-promotion-summary.preserves" \
                  --signed-key-ledger "$out/signed-keyring" \
                  --signed-key-id local-release-key-v1 \
                  --signed-trust-root local-release-trust-root \
                  --signed-signer local-release-signer \
                  | tee "$out/release-promotion-summary.txt"
                grep -q 'decision=pass' "$out/release-promotion-summary.txt"
                molten dogfood release-export \
                  --output-path "$out" \
                  --out "$out/release-evidence.tar.zst" \
                  --manifest-out "$out/release-export-manifest.preserves" \
                  | tee "$out/release-export.txt"
                grep -q 'release-export manifest=' "$out/release-export.txt"
                molten dogfood release-export-verify \
                  --bundle "$out/release-evidence.tar.zst" \
                  --receipt-out "$out/release-export-verify.preserves" \
                  | tee "$out/release-export-verify.txt"
                grep -q 'decision=pass' "$out/release-export-verify.txt"
              '';

          nixos-vm-multinode = pkgs.testers.runNixOSTest {
            name = "molten-nixos-vm-multinode";
            nodes = {
              node_a = moltenVmNodeModule "node-a";
              node_b = moltenVmNodeModule "node-b";
            };
            testScript = ''
              start_all()
              for machine in [node_a, node_b]:
                  machine.wait_for_unit("molten-node.service")
                  machine.succeed("test -s /var/lib/molten/vm-evidence/startup.preserves")
                  machine.succeed("test -s /var/lib/molten/vm-evidence/health.preserves")
                  machine.succeed("test -s /var/lib/molten/vm-evidence/control-loop.preserves")
                  machine.succeed("test -s /var/lib/molten/vm-evidence/heartbeat.preserves")
                  machine.succeed("grep -q node-startup-receipt-v1 /var/lib/molten/vm-evidence/startup.preserves")
                  machine.succeed("grep -q node-health-receipt-v1 /var/lib/molten/vm-evidence/health.preserves")
                  machine.succeed("grep -q node-control-loop-receipt-v1 /var/lib/molten/vm-evidence/control-loop.preserves")

              node_a.succeed("getent hosts node-b")
              node_b.succeed("getent hosts node-a")
              node_a.succeed("ping -c 1 node-b")
              node_b.succeed("ping -c 1 node-a")

              node_b.succeed("""
                set -euo pipefail
                cd '${sourceForConfigChecks}'
                root=/var/lib/molten-cross/node-b
                evidence=/var/lib/molten/vm-evidence/live-control
                rm -rf "$root" "$evidence"
                mkdir -p "$evidence"
                molten node init \
                  --state-root "$root" \
                  --node-id node:node-b \
                  --config-out "$evidence/node-config.preserves" \
                  --identity-receipt-out "$evidence/identity.preserves" \
                  > "$evidence/init.txt"
                molten node run \
                  --state-root "$root" \
                  --startup-out "$evidence/startup.preserves" \
                  > "$evidence/run.txt"
                molten node serve \
                  --state-root "$root" \
                  --live-iroh \
                  --live-max-events 1 \
                  --live-event-timeout-ms 1 \
                  --max-requests-per-tick 1 \
                  --service-receipt-out "$evidence/live-service.preserves" \
                  --live-ticket-out "$evidence/ticket.preserves" \
                  --receipt-out "$evidence/listener.preserves" \
                  > "$evidence/live-serve.txt"
                molten node live-peer-admit \
                  --state-root "$root" \
                  --peer node:node-a \
                  --receipt-out "$evidence/peer-admission.preserves" \
                  "$evidence/ticket.preserves" \
                  > "$evidence/admit.txt"
                molten node authority-grant-fixture \
                  --state-root "$root" \
                  --peer node:node-a \
                  --node node:node-b \
                  --operation status \
                  --target-scope '*' \
                  --resource-scope '*' \
                  --out "$evidence/authority-grant.preserves" \
                  > "$evidence/authority.txt"
                peer_summary=$(molten test nixos-vm show "$evidence/peer-admission.preserves")
                peer_ref=''${peer_summary#* ref=}
                peer_ref=''${peer_ref%% *}
                authority_summary=$(molten test nixos-vm show "$evidence/authority-grant.preserves")
                authority_ref=''${authority_summary#* ref=}
                authority_ref=''${authority_ref%% *}
                printf '%s\n' "$peer_ref" > "$evidence/peer.ref"
                printf '%s\n' "$authority_ref" > "$evidence/authority.ref"
                molten node control-request \
                  --operation status \
                  --authority "$authority_ref" \
                  --policy "$peer_ref" \
                  --resource "$authority_ref" \
                  --out "$evidence/request.preserves" \
                  > "$evidence/request.txt"
                molten node control-ingress-live-loopback \
                  --state-root "$root" \
                  "$evidence/request.preserves" \
                  --from-peer node:node-a \
                  --to-node node:node-b \
                  --sequence 1 \
                  --peer-bootstrap "$peer_ref" \
                  --authority "$authority_ref" \
                  --policy "$peer_ref" \
                  --resource "$authority_ref" \
                  --publish-receipt-out "$evidence/live-publish.preserves" \
                  --receive-receipt-out "$evidence/live-receive.preserves" \
                  > "$evidence/live-loopback.txt"
                molten node run-loop \
                  --state-root "$root" \
                  --max-requests 1 \
                  --receipt-out "$evidence/live-control-loop.preserves" \
                  --heartbeat-out "$evidence/live-heartbeat.preserves" \
                  > "$evidence/live-run-loop.txt"
                cp "$root"/control/iroh-ingress/receipts/*.deliver.receipt.preserves "$evidence/ingress.preserves"
                cp "$root"/control/inbox/*.queue-receipt.preserves "$evidence/queue.preserves"
                cp "$root"/control/outbox/*.control-receipt.preserves "$evidence/control.preserves"
                molten node live-workflow-bundle-export \
                  --ticket "$evidence/ticket.preserves" \
                  --peer-admission "$evidence/peer-admission.preserves" \
                  --authority-grant "$evidence/authority-grant.preserves" \
                  --receipt "$evidence/listener.preserves" \
                  --receipt "$evidence/live-service.preserves" \
                  --receipt "$evidence/live-receive.preserves" \
                  --out "$evidence/bundle.preserves" \
                  --receipt-out "$evidence/bundle-export.preserves" \
                  > "$evidence/bundle-export.txt"
                grep -q node-control-live-workflow-bundle-v1 "$evidence/bundle.preserves"
                grep -q node-control-live-transport-receipt-v1 "$evidence/live-receive.preserves"
              """)
              node_a.succeed("mkdir -p /var/lib/molten/vm-evidence/live-control")
              for artifact in [
                  "ticket.preserves",
                  "peer-admission.preserves",
                  "authority-grant.preserves",
                  "bundle.preserves",
                  "request.preserves",
                  "ingress.preserves",
                  "queue.preserves",
                  "control.preserves",
                  "peer.ref",
                  "authority.ref",
                  "live-loopback.txt",
              ]:
                  content = node_b.succeed(f"cat /var/lib/molten/vm-evidence/live-control/{artifact}")
                  node_a.succeed(f"cat > /var/lib/molten/vm-evidence/live-control/{artifact} <<'EOF'\n" + content + "\nEOF")
              node_a.succeed("""
                set -euo pipefail
                cd '${sourceForConfigChecks}'
                root=/var/lib/molten-cross/node-a
                evidence=/var/lib/molten/vm-evidence/live-control
                peer_ref=$(cat "$evidence/peer.ref")
                authority_ref=$(cat "$evidence/authority.ref")
                rm -rf "$root"
                molten node init \
                  --state-root "$root" \
                  --node-id node:node-a \
                  --config-out "$evidence/sender-node-config.preserves" \
                  --identity-receipt-out "$evidence/sender-identity.preserves" \
                  > "$evidence/sender-init.txt"
                molten node run \
                  --state-root "$root" \
                  --startup-out "$evidence/sender-startup.preserves" \
                  > "$evidence/sender-run.txt"
                molten node live-workflow-bundle-verify \
                  "$evidence/bundle.preserves" \
                  --expected-node node:node-b \
                  --expected-peer node:node-a \
                  --operation status \
                  --receipt-out "$evidence/verify.preserves" \
                  > "$evidence/verify.txt"
                molten node live-workflow-bundle-gate \
                  "$evidence/bundle.preserves" \
                  --verify-receipt "$evidence/verify.preserves" \
                  --require-verify-receipt \
                  --expected-node node:node-b \
                  --expected-peer node:node-a \
                  --operation status \
                  --receipt-out "$evidence/gate.preserves" \
                  > "$evidence/gate.txt"
                molten node live-workflow-bundle-apply \
                  --state-root "$root" \
                  "$evidence/bundle.preserves" \
                  --gate-receipt "$evidence/gate.preserves" \
                  --require-gate-receipt \
                  --request "$evidence/request.preserves" \
                  --from-peer node:node-a \
                  --sequence 1 \
                  --peer-bootstrap "$peer_ref" \
                  --authority "$authority_ref" \
                  --policy "$peer_ref" \
                  --resource "$authority_ref" \
                  --expected-node node:node-b \
                  --expected-peer node:node-a \
                  --operation status \
                  --receipt-out "$evidence/apply.preserves" \
                  > "$evidence/apply.txt"
                molten node live-workflow-bundle-reconcile \
                  "$evidence/apply.preserves" \
                  --ingress-receipt "$evidence/ingress.preserves" \
                  --queue-receipt "$evidence/queue.preserves" \
                  --control-receipt "$evidence/control.preserves" \
                  --receipt-out "$evidence/reconcile.preserves" \
                  > "$evidence/reconcile.txt"
                molten node live-workflow-bundle-ack-export \
                  "$evidence/apply.preserves" \
                  --ingress-receipt "$evidence/ingress.preserves" \
                  --queue-receipt "$evidence/queue.preserves" \
                  --control-receipt "$evidence/control.preserves" \
                  --reconcile-receipt "$evidence/reconcile.preserves" \
                  --out "$evidence/ack.preserves" \
                  --receipt-out "$evidence/ack-export.preserves" \
                  > "$evidence/ack-export.txt"
                molten node live-workflow-bundle-ack-import \
                  --state-root "$root" \
                  "$evidence/ack.preserves" \
                  --receipt-out "$evidence/ack-import.preserves" \
                  > "$evidence/ack-import.txt"
                molten node live-workflow-bundle-protocol-gate \
                  "$evidence/bundle.preserves" \
                  --gate-receipt "$evidence/gate.preserves" \
                  --apply-receipt "$evidence/apply.preserves" \
                  --reconcile-receipt "$evidence/reconcile.preserves" \
                  --ack "$evidence/ack.preserves" \
                  --receipt-out "$evidence/protocol-gate.preserves" \
                  > "$evidence/protocol-gate.txt"
                grep -q 'decision "pass"' "$evidence/apply.preserves"
                grep -q 'decision "pass"' "$evidence/reconcile.preserves"
                grep -q 'decision "pass"' "$evidence/ack-export.preserves"
                grep -q 'decision "pass"' "$evidence/ack-import.preserves"
                grep -q 'decision "pass"' "$evidence/protocol-gate.preserves"
              """)

              node_a.succeed("""
                set -euo pipefail
                cd '${sourceForConfigChecks}'
                evidence=/var/lib/molten/vm-evidence/service-job
                rm -rf "$evidence"
                mkdir -p "$evidence"
                make_ref() {
                  label=$1
                  file="$evidence/ref-$label.preserves"
                  printf '<vm-synthetic-ref "%s">\n' "$label" > "$file"
                  summary=$(molten test nixos-vm show "$file")
                  ref=''${summary#* ref=}
                  ref=''${ref%% *}
                  printf '%s\n' "$ref"
                }
                printf '<vm-service-message "node_a" "node_b">\n' > "$evidence/remote-payload.preserves"
                remote_policy_ref=$(make_ref remote-policy)
                remote_capability_ref=$(make_ref remote-capability)
                remote_evidence_ref=$(make_ref remote-evidence)
                molten test remote envelope build \
                  --from-peer node_a \
                  --from-actor service-client \
                  --to-peer node_b \
                  --topic molten.vm.service \
                  --operation message \
                  --payload "$evidence/remote-payload.preserves" \
                  --capability-ref "$remote_capability_ref" \
                  --evidence-ref "$remote_evidence_ref" \
                  --out "$evidence/remote-envelope.preserves" \
                  > "$evidence/remote-envelope.txt"
                remote_summary=$(molten test nixos-vm show "$evidence/remote-envelope.preserves")
                remote_ref=''${remote_summary#* ref=}
                remote_ref=''${remote_ref%% *}
                printf '%s\n' "$remote_ref" > "$evidence/remote-envelope.ref"
                printf 'echo' > "$evidence/executable.bin"
                printf 'hello' > "$evidence/input.bin"
                exe_summary=$(molten test chunk put "$evidence/executable.bin" \
                  --store "$evidence/source-chunks" \
                  --kind job-executable \
                  --manifest-out "$evidence/executable-manifest.preserves" \
                  --receipt-out "$evidence/executable-put.preserves")
                exe_manifest=''${exe_summary#* manifest=}
                exe_manifest=''${exe_manifest%% *}
                input_summary=$(molten test chunk put "$evidence/input.bin" \
                  --store "$evidence/source-chunks" \
                  --kind job-input \
                  --manifest-out "$evidence/input-manifest.preserves" \
                  --receipt-out "$evidence/input-put.preserves")
                input_manifest=''${input_summary#* manifest=}
                input_manifest=''${input_manifest%% *}
                operation_ref=$(make_ref job-operation)
                policy_ref=$(make_ref job-policy)
                provenance_ref=$(make_ref job-provenance)
                effect_ref=$(make_ref job-effect)
                authority_ref=$(make_ref job-authority)
                molten test job ref-submit \
                  --job-id job:vm-echo \
                  --operation-id "$operation_ref" \
                  --executable "$exe_manifest@4@elf-executable" \
                  --input "$input_manifest@5@bytes" \
                  --output-mode chunk-manifest \
                  --handler-profile local-echo-v1 \
                  --context-ref "$authority_ref" \
                  --policy-ref "$policy_ref" \
                  --provenance-ref "$provenance_ref" \
                  --effect-ref "$effect_ref" \
                  --evidence-ref "$remote_ref" \
                  --out "$evidence/job-submission.preserves" \
                  > "$evidence/job-ref-submit.txt"
                coord_policy_ref=$(make_ref coordination-policy)
                coord_resource_ref=$(make_ref coordination-resource)
                coord_authority_ref=$(make_ref coordination-authority)
                coord_operation_ref=$(make_ref coordination-operation)
                printf '%s\n' "$coord_policy_ref" > "$evidence/coord-policy.ref"
                printf '%s\n' "$coord_resource_ref" > "$evidence/coord-resource.ref"
                printf '%s\n' "$coord_authority_ref" > "$evidence/coord-authority.ref"
                printf '<coordination-item "vm-job-worker">\n' > "$evidence/coord-payload.preserves"
                molten test coordination request \
                  --service queue \
                  --operation enqueue \
                  --key queue:vm-jobs \
                  --client-session node-a \
                  --operation-id-ref "$coord_operation_ref" \
                  --payload "$evidence/coord-payload.preserves" \
                  --authority-ref "$coord_authority_ref" \
                  --policy-ref "$coord_policy_ref" \
                  --resource-ref "$coord_resource_ref" \
                  --out "$evidence/coord-request.preserves" \
                  > "$evidence/coord-request.txt"
              """)
              node_b.succeed("mkdir -p /var/lib/molten/vm-evidence/service-job")
              for artifact in [
                  "remote-envelope.preserves",
                  "job-submission.preserves",
                  "coord-request.preserves",
                  "coord-policy.ref",
                  "coord-resource.ref",
                  "coord-authority.ref",
              ]:
                  content = node_a.succeed(f"cat /var/lib/molten/vm-evidence/service-job/{artifact}")
                  node_b.succeed(f"cat > /var/lib/molten/vm-evidence/service-job/{artifact} <<'EOF'\n" + content + "\nEOF")
              node_b.succeed("""
                set -euo pipefail
                cd '${sourceForConfigChecks}'
                evidence=/var/lib/molten/vm-evidence/service-job
                rm -rf "$evidence/target-chunks" "$evidence/job-ledger" "$evidence/coord-apply" "$evidence/remote-transport"
                printf 'echo' > "$evidence/executable.bin"
                printf 'hello' > "$evidence/input.bin"
                molten test remote publish-local \
                  --transport-root "$evidence/remote-transport" \
                  --envelope "$evidence/remote-envelope.preserves" \
                  --node node_a \
                  --receipt-out "$evidence/remote-publish.preserves" \
                  > "$evidence/remote-publish.txt"
                remote_summary=$(molten test nixos-vm show "$evidence/remote-envelope.preserves")
                remote_ref=''${remote_summary#* ref=}
                remote_ref=''${remote_ref%% *}
                molten test remote deliver-local \
                  --transport-root "$evidence/remote-transport" \
                  --topic molten.vm.service \
                  --envelope-ref "$remote_ref" \
                  --receiver-peer node_b \
                  --out "$evidence/remote-delivered.preserves" \
                  --receipt-out "$evidence/remote-deliver.preserves" \
                  > "$evidence/remote-deliver.txt"
                molten test chunk put "$evidence/executable.bin" \
                  --store "$evidence/target-chunks" \
                  --kind job-executable \
                  --manifest-out "$evidence/target-executable-manifest.preserves" \
                  --receipt-out "$evidence/target-executable-put.preserves" \
                  > "$evidence/target-executable-put.txt"
                molten test chunk put "$evidence/input.bin" \
                  --store "$evidence/target-chunks" \
                  --kind job-input \
                  --manifest-out "$evidence/target-input-manifest.preserves" \
                  --receipt-out "$evidence/target-input-put.preserves" \
                  > "$evidence/target-input-put.txt"
                molten test job ref-execute "$evidence/job-submission.preserves" \
                  --chunks "$evidence/target-chunks" \
                  --ledger "$evidence/job-ledger" \
                  --receipt-out "$evidence/job-ref.receipt.preserves" \
                  > "$evidence/job-ref-execute.txt"
                coord_policy_ref=$(cat "$evidence/coord-policy.ref")
                coord_resource_ref=$(cat "$evidence/coord-resource.ref")
                molten test coordination manifest \
                  --service queue \
                  --policy-ref "$coord_policy_ref" \
                  --resource-ref "$coord_resource_ref" \
                  --out "$evidence/coord-manifest.preserves" \
                  > "$evidence/coord-manifest.txt"
                molten test coordination apply \
                  --manifest "$evidence/coord-manifest.preserves" \
                  --request "$evidence/coord-request.preserves" \
                  --out "$evidence/coord-apply" \
                  > "$evidence/coord-apply.txt"
                grep -q remote-dataspace-transport-receipt-v1 "$evidence/remote-deliver.preserves"
                grep -q 'decision "pass"' "$evidence/job-ref.receipt.preserves"
                grep -q coordination-apply-report-v1 "$evidence/coord-apply/report.preserves"
              """)
              node_a.succeed("mkdir -p /var/lib/molten/vm-evidence/service-job/coord-apply")
              for artifact in [
                  "remote-deliver.preserves",
                  "remote-deliver.txt",
                  "job-ref.receipt.preserves",
                  "job-ref-execute.txt",
                  "target-executable-put.preserves",
                  "target-input-put.preserves",
                  "coord-apply/report.preserves",
                  "coord-apply.txt",
              ]:
                  content = node_b.succeed(f"cat /var/lib/molten/vm-evidence/service-job/{artifact}")
                  node_a.succeed(f"cat > /var/lib/molten/vm-evidence/service-job/{artifact} <<'EOF'\n" + content + "\nEOF")

              node_b.succeed("""
                molten node control-request \
                  --operation status \
                  --out /var/lib/molten/vm-evidence/restart-status-request.preserves
                molten node control-submit \
                  --state-root /var/lib/molten \
                  /var/lib/molten/vm-evidence/restart-status-request.preserves \
                  --receipt-out /var/lib/molten/vm-evidence/restart-queue.preserves
              """)
              node_b.succeed("systemctl restart molten-node.service")
              node_b.wait_for_unit("molten-node.service")
              node_b.succeed("grep -q processed=1 /var/lib/molten/vm-evidence/run-loop.txt")
              node_b.succeed("grep -q node-control-loop-receipt-v1 /var/lib/molten/vm-evidence/control-loop.preserves")

              node_a.succeed("systemctl stop molten-node.service")
              node_b.succeed("systemctl stop molten-node.service")
              for machine in [node_a, node_b]:
                  machine.succeed("test -s /var/lib/molten/vm-evidence/shutdown.preserves")
                  machine.succeed("grep -q node-shutdown-receipt-v1 /var/lib/molten/vm-evidence/shutdown.preserves")

              node_a.succeed("""
                molten test nixos-vm topology \
                  --node node_a \
                  --node node_b \
                  --package-ref 'store:${moltenPkg}' \
                  --package-path '${moltenPkg}' \
                  --network nixos-test-private \
                  --nix-input 'source:${sourceForConfigChecks}' \
                  --caveat 'vm evidence is platform integration evidence only' \
                  --out /var/lib/molten/vm-evidence/topology.preserves
              """)
              for machine, node_name in [(node_a, "node_a"), (node_b, "node_b")]:
                  machine.succeed(f"""
                    molten test nixos-vm node-evidence \
                      --node {node_name} \
                      --state-root /var/lib/molten \
                      --identity /var/lib/molten/vm-evidence/identity.preserves \
                      --startup /var/lib/molten/vm-evidence/startup.preserves \
                      --health /var/lib/molten/vm-evidence/health.preserves \
                      --control-loop /var/lib/molten/vm-evidence/control-loop.preserves \
                      --heartbeat /var/lib/molten/vm-evidence/heartbeat.preserves \
                      --shutdown /var/lib/molten/vm-evidence/shutdown.preserves \
                      --log /var/lib/molten/vm-evidence/init.txt \
                      --log /var/lib/molten/vm-evidence/run.txt \
                      --log /var/lib/molten/vm-evidence/status.txt \
                      --log /var/lib/molten/vm-evidence/run-loop.txt \
                      --out /var/lib/molten/vm-evidence/node-evidence.preserves
                  """)
              node_b_evidence = node_b.succeed("cat /var/lib/molten/vm-evidence/node-evidence.preserves")
              node_a.succeed("cat > /var/lib/molten/vm-evidence/node-b-evidence.preserves <<'EOF'\n" + node_b_evidence + "\nEOF")
              for artifact in [
                  "restart-queue.preserves",
                  "control-loop.preserves",
                  "heartbeat.preserves",
                  "shutdown.preserves",
              ]:
                  content = node_b.succeed(f"cat /var/lib/molten/vm-evidence/{artifact}")
                  node_a.succeed(f"cat > /var/lib/molten/vm-evidence/node-b-{artifact} <<'EOF'\n" + content + "\nEOF")
              node_a.succeed("""
                protocol_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/live-control/protocol-gate.preserves)
                protocol_ref=''${protocol_summary#* ref=}
                protocol_ref=''${protocol_ref%% *}
                reconcile_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/live-control/reconcile.preserves)
                reconcile_ref=''${reconcile_summary#* ref=}
                reconcile_ref=''${reconcile_ref%% *}
                ack_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/live-control/ack-export.preserves)
                ack_ref=''${ack_summary#* ref=}
                ack_ref=''${ack_ref%% *}
                remote_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/service-job/remote-deliver.preserves)
                remote_ref=''${remote_summary#* ref=}
                remote_ref=''${remote_ref%% *}
                job_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/service-job/job-ref.receipt.preserves)
                job_ref=''${job_summary#* ref=}
                job_ref=''${job_ref%% *}
                coordination_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/service-job/coord-apply/report.preserves)
                coordination_ref=''${coordination_summary#* ref=}
                coordination_ref=''${coordination_ref%% *}
                ticket_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/live-control/ticket.preserves)
                ticket_ref=''${ticket_summary#* ref=}
                ticket_ref=''${ticket_ref%% *}
                peer_admission_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/live-control/peer-admission.preserves)
                peer_admission_ref=''${peer_admission_summary#* ref=}
                peer_admission_ref=''${peer_admission_ref%% *}
                authority_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/live-control/authority-grant.preserves)
                authority_ref=''${authority_summary#* ref=}
                authority_ref=''${authority_ref%% *}
                fault_dir=/var/lib/molten/vm-evidence/prod-soak-faults
                mkdir -p "$fault_dir"
                molten node live-workflow-bundle-gate \
                  /var/lib/molten/vm-evidence/live-control/bundle.preserves \
                  --expected-peer node:stale-peer \
                  --receipt-out "$fault_dir/stale-ticket-gate.preserves" \
                  > "$fault_dir/stale-ticket-gate.txt"
                grep -q 'decision "deny"' "$fault_dir/stale-ticket-gate.preserves"
                stale_summary=$(molten test prod-soak show "$fault_dir/stale-ticket-gate.preserves")
                stale_denial_ref=''${stale_summary#* ref=}
                stale_denial_ref=''${stale_denial_ref%% *}
                molten node live-workflow-bundle-gate \
                  /var/lib/molten/vm-evidence/live-control/bundle.preserves \
                  --expected-node node:wrong-authority \
                  --receipt-out "$fault_dir/wrong-authority-gate.preserves" \
                  > "$fault_dir/wrong-authority-gate.txt"
                grep -q 'decision "deny"' "$fault_dir/wrong-authority-gate.preserves"
                wrong_authority_summary=$(molten test prod-soak show "$fault_dir/wrong-authority-gate.preserves")
                wrong_authority_denial_ref=''${wrong_authority_summary#* ref=}
                wrong_authority_denial_ref=''${wrong_authority_denial_ref%% *}
                fault_case() {
                  kind=$1
                  expected=$2
                  denial_ref=''${3:-}
                  out="$fault_dir/$kind.preserves"
                  if [ -n "$denial_ref" ]; then
                    molten test prod-soak fault-case \
                      --scenario network-transport-fault-matrix \
                      --fault-kind "$kind" \
                      --injection simulated-vm-fault \
                      --expected-outcome "$expected" \
                      --evidence-ref "$protocol_ref" \
                      --evidence-ref "$remote_ref" \
                      --denial-ref "$denial_ref" \
                      --decision pass \
                      --replay-status simulated-fault \
                      --caveat 'fault evidence is simulated diagnostic evidence for pilot scoping' \
                      --out "$out"
                  else
                    molten test prod-soak fault-case \
                      --scenario network-transport-fault-matrix \
                      --fault-kind "$kind" \
                      --injection simulated-vm-fault \
                      --expected-outcome "$expected" \
                      --evidence-ref "$protocol_ref" \
                      --evidence-ref "$remote_ref" \
                      --decision pass \
                      --replay-status simulated-fault \
                      --caveat 'fault evidence is simulated diagnostic evidence for pilot scoping' \
                      --out "$out"
                  fi
                }
                fault_case delay bounded-diagnostic
                fault_case drop bounded-diagnostic
                fault_case partition bounded-diagnostic
                fault_case rejoin bounded-diagnostic
                fault_case stale-ticket deny-before-side-effects "$stale_denial_ref"
                fault_case wrong-authority deny-before-side-effects "$wrong_authority_denial_ref"
                fault_case duplicate-operation idempotent-replay
                fault_case conflicting-operation-id deny-before-side-effects "$wrong_authority_denial_ref"
                fault_case corrupted-transport-receipt deny-before-side-effects "$stale_denial_ref"
                molten test prod-soak fault-matrix \
                  --scenario network-transport-fault-matrix \
                  --fault-case "$fault_dir/delay.preserves" --fault-kind delay \
                  --fault-case "$fault_dir/drop.preserves" --fault-kind drop \
                  --fault-case "$fault_dir/partition.preserves" --fault-kind partition \
                  --fault-case "$fault_dir/rejoin.preserves" --fault-kind rejoin \
                  --fault-case "$fault_dir/stale-ticket.preserves" --fault-kind stale-ticket \
                  --fault-case "$fault_dir/wrong-authority.preserves" --fault-kind wrong-authority \
                  --fault-case "$fault_dir/duplicate-operation.preserves" --fault-kind duplicate-operation \
                  --fault-case "$fault_dir/conflicting-operation-id.preserves" --fault-kind conflicting-operation-id \
                  --fault-case "$fault_dir/corrupted-transport-receipt.preserves" --fault-kind corrupted-transport-receipt \
                  --decision pass \
                  --caveat 'fault matrix evidence is diagnostic and does not prove transport correctness beyond this topology' \
                  --out "$fault_dir/matrix.preserves"
                fault_matrix_summary=$(molten test prod-soak show "$fault_dir/matrix.preserves")
                fault_matrix_ref=''${fault_matrix_summary#* ref=}
                fault_matrix_ref=''${fault_matrix_ref%% *}
                restart_queue_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/node-b-restart-queue.preserves)
                restart_queue_ref=''${restart_queue_summary#* ref=}
                restart_queue_ref=''${restart_queue_ref%% *}
                recovery_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/node-b-control-loop.preserves)
                recovery_ref=''${recovery_summary#* ref=}
                recovery_ref=''${recovery_ref%% *}
                executable_put_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/service-job/target-executable-put.preserves)
                executable_put_ref=''${executable_put_summary#* ref=}
                executable_put_ref=''${executable_put_ref%% *}
                input_put_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/service-job/target-input-put.preserves)
                input_put_ref=''${input_put_summary#* ref=}
                input_put_ref=''${input_put_ref%% *}
                molten test prod-soak durability \
                  --scenario restart-durability \
                  --queued-control-ref "$restart_queue_ref" \
                  --recovery-ref "$recovery_ref" \
                  --ledger-ref "$job_ref" \
                  --chunk-ref "$executable_put_ref" \
                  --chunk-ref "$input_put_ref" \
                  --retention-ref "$job_ref" \
                  --retention-ref "$coordination_ref" \
                  --decision pass \
                  --caveat 'durability evidence is scoped to this VM topology and remains diagnostic' \
                  --out /var/lib/molten/vm-evidence/prod-soak-durability.preserves
                durability_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/prod-soak-durability.preserves)
                durability_ref=''${durability_summary#* ref=}
                durability_ref=''${durability_ref%% *}
                receipt_bytes=$(stat -c%s "$fault_dir/matrix.preserves")
                store_bytes=$(du -sb /var/lib/molten/vm-evidence | cut -f1)
                molten test prod-soak resource-envelope \
                  --scenario pilot-resource-envelope \
                  --queue-depth 1 \
                  --max-queue-depth 8 \
                  --receipt-bytes "$receipt_bytes" \
                  --max-receipt-bytes 1000000 \
                  --store-bytes "$store_bytes" \
                  --max-store-bytes 100000000 \
                  --delivery-latency-ms 0 \
                  --max-delivery-latency-ms 60000 \
                  --recovery-time-ms 0 \
                  --max-recovery-time-ms 60000 \
                  --pressure-ref "$fault_matrix_ref" \
                  --denial-ref "$stale_denial_ref" \
                  --denial-ref "$wrong_authority_denial_ref" \
                  --decision pass \
                  --caveat 'resource metrics are single-VM pilot bounds and are not SLO evidence' \
                  --out /var/lib/molten/vm-evidence/prod-soak-resource-envelope.preserves
                resource_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/prod-soak-resource-envelope.preserves)
                resource_ref=''${resource_summary#* ref=}
                resource_ref=''${resource_ref%% *}
                vm_fault_dir=/var/lib/molten/vm-evidence/vm-faults
                mkdir -p "$vm_fault_dir"
                fault_duration_millis=1000
                printf '<vm-fault-injection "network-control-unavailable">\n' > "$vm_fault_dir/network-unavailable.preserves"
                printf '%s\n' 'network control command unavailable in this VM image' > "$vm_fault_dir/network-unavailable.log"
                molten test nixos-vm fault-descriptor \
                  --fault-id network-partition-node-a-node-b \
                  --topology /var/lib/molten/vm-evidence/topology.preserves \
                  --target-node node_a \
                  --target-link node_a-to-node_b \
                  --fault-kind network-partition \
                  --command-profile nixos-test-driver-network-control \
                  --expected-outcome unavailable \
                  --duration-millis "$fault_duration_millis" \
                  --trigger network-control-preflight \
                  --preflight /var/lib/molten/vm-evidence/topology.preserves \
                  --caveat 'network fault execution is unavailable unless VM image exposes network-control tools' \
                  --out "$vm_fault_dir/network-partition.descriptor.preserves"
                molten test nixos-vm fault-receipt \
                  --descriptor "$vm_fault_dir/network-partition.descriptor.preserves" \
                  --decision unavailable \
                  --host-support unavailable \
                  --pre-fault /var/lib/molten/vm-evidence/topology.preserves \
                  --injection "$vm_fault_dir/network-unavailable.preserves" \
                  --post-fault /var/lib/molten/vm-evidence/node-evidence.preserves \
                  --replay-status unavailable-network-control \
                  --diagnostic 'network-control support unavailable; no pass evidence minted' \
                  --log "$vm_fault_dir/network-unavailable.log" \
                  --caveat 'unavailable VM fault evidence cannot satisfy pass claims' \
                  --out "$vm_fault_dir/network-partition.receipt.preserves"
                molten test nixos-vm fault-descriptor \
                  --fault-id duplicate-send-after-restart \
                  --topology /var/lib/molten/vm-evidence/topology.preserves \
                  --target-node node_b \
                  --fault-kind duplicate-send-after-restart \
                  --command-profile systemd-restart-window \
                  --expected-outcome idempotent-recovery \
                  --duration-millis "$fault_duration_millis" \
                  --trigger queued-control-restart \
                  --preflight /var/lib/molten/vm-evidence/node-b-restart-queue.preserves \
                  --caveat 'restart fault evidence is scoped to this VM topology' \
                  --out "$vm_fault_dir/restart.descriptor.preserves"
                molten test nixos-vm fault-receipt \
                  --descriptor "$vm_fault_dir/restart.descriptor.preserves" \
                  --decision pass \
                  --host-support supported \
                  --pre-fault /var/lib/molten/vm-evidence/node-b-restart-queue.preserves \
                  --injection /var/lib/molten/vm-evidence/node-b-control-loop.preserves \
                  --child /var/lib/molten/vm-evidence/node-b-control-loop.preserves \
                  --post-fault /var/lib/molten/vm-evidence/node-b-heartbeat.preserves \
                  --replay-status restart-window-observed \
                  --log /var/lib/molten/vm-evidence/run-loop.txt \
                  --caveat 'restart evidence demonstrates bounded idempotent recovery only' \
                  --out "$vm_fault_dir/restart.receipt.preserves"
                printf '<vm-fault-injection "permission-denied-state-root-denial">\n' > "$vm_fault_dir/storage-denial.preserves"
                printf '%s\n' 'permission denied before mutation fixture' > "$vm_fault_dir/storage-denial.log"
                molten test nixos-vm fault-descriptor \
                  --fault-id permission-denied-state-root \
                  --topology /var/lib/molten/vm-evidence/topology.preserves \
                  --target-node node_b \
                  --fault-kind permission-denied-state-root \
                  --command-profile filesystem-state-root \
                  --expected-outcome deny-before-side-effects \
                  --duration-millis "$fault_duration_millis" \
                  --trigger state-root-write-preflight \
                  --preflight /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                  --caveat 'storage fault evidence is bounded and diagnostic outside canonical receipts' \
                  --out "$vm_fault_dir/storage.descriptor.preserves"
                molten test nixos-vm fault-receipt \
                  --descriptor "$vm_fault_dir/storage.descriptor.preserves" \
                  --decision deny \
                  --host-support supported \
                  --pre-fault /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                  --injection "$vm_fault_dir/storage-denial.preserves" \
                  --post-fault /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                  --replay-status denial-observed-before-mutation \
                  --diagnostic 'permission denied before mutation' \
                  --log "$vm_fault_dir/storage-denial.log" \
                  --caveat 'deny receipt is authoritative; logs are diagnostic only' \
                  --out "$vm_fault_dir/storage.receipt.preserves"
                molten test nixos-vm fault-validate \
                  --topology /var/lib/molten/vm-evidence/topology.preserves \
                  --descriptor "$vm_fault_dir/network-partition.descriptor.preserves" \
                  --descriptor "$vm_fault_dir/restart.descriptor.preserves" \
                  --descriptor "$vm_fault_dir/storage.descriptor.preserves" \
                  --receipt "$vm_fault_dir/network-partition.receipt.preserves" \
                  --receipt "$vm_fault_dir/restart.receipt.preserves" \
                  --receipt "$vm_fault_dir/storage.receipt.preserves" \
                  --out "$vm_fault_dir/validation.preserves"
                vm_fault_validation_summary=$(molten test nixos-vm show "$vm_fault_dir/validation.preserves")
                vm_fault_validation_ref=''${vm_fault_validation_summary#* ref=}
                vm_fault_validation_ref=''${vm_fault_validation_ref%% *}
                molten test prod-soak evidence-export \
                  --node node_a \
                  --node-evidence /var/lib/molten/vm-evidence/node-evidence.preserves \
                  --artifact /var/lib/molten/vm-evidence/live-control/protocol-gate.preserves \
                  --artifact /var/lib/molten/vm-evidence/live-control/ack-export.preserves \
                  --artifact /var/lib/molten/vm-evidence/service-job/remote-deliver.preserves \
                  --artifact /var/lib/molten/vm-evidence/prod-soak-durability.preserves \
                  --artifact /var/lib/molten/vm-evidence/prod-soak-resource-envelope.preserves \
                  --log /var/lib/molten/vm-evidence/live-control/protocol-gate.txt \
                  --out /var/lib/molten/vm-evidence/prod-soak-node-a-export.preserves
                molten test prod-soak evidence-export \
                  --node node_b \
                  --node-evidence /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                  --artifact /var/lib/molten/vm-evidence/service-job/job-ref.receipt.preserves \
                  --artifact /var/lib/molten/vm-evidence/service-job/coord-apply/report.preserves \
                  --artifact /var/lib/molten/vm-evidence/live-control/reconcile.preserves \
                  --log /var/lib/molten/vm-evidence/service-job/job-ref-execute.txt \
                  --out /var/lib/molten/vm-evidence/prod-soak-node-b-export.preserves
                molten test prod-soak run-receipt \
                  --topology /var/lib/molten/vm-evidence/topology.preserves \
                  --node-evidence /var/lib/molten/vm-evidence/node-evidence.preserves \
                  --node-evidence /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                  --scenario production-shaped-vm-live-workflow \
                  --fault-profile network-transport-fault-matrix \
                  --peer-ticket-ref "$ticket_ref" \
                  --peer-ticket-ref "$peer_admission_ref" \
                  --peer-ticket-ref "$authority_ref" \
                  --node-control-ref "$protocol_ref" \
                  --node-control-ref "$reconcile_ref" \
                  --node-control-ref "$ack_ref" \
                  --remote-service-ref "$remote_ref" \
                  --job-ref "$job_ref" \
                  --coordination-ref "$coordination_ref" \
                  --fault-ref "$fault_matrix_ref" \
                  --durability-ref "$durability_ref" \
                  --resource-ref "$resource_ref" \
                  --evidence-export /var/lib/molten/vm-evidence/prod-soak-node-a-export.preserves \
                  --evidence-export /var/lib/molten/vm-evidence/prod-soak-node-b-export.preserves \
                  --log /var/lib/molten/vm-evidence/live-control/protocol-gate.txt \
                  --log /var/lib/molten/vm-evidence/service-job/remote-deliver.txt \
                  --log /var/lib/molten/vm-evidence/service-job/job-ref-execute.txt \
                  --log /var/lib/molten/vm-evidence/service-job/coord-apply.txt \
                  --log /var/lib/molten/vm-evidence/prod-soak-faults/stale-ticket-gate.txt \
                  --decision pass \
                  --replay-status non-replayable-live-observations \
                  --caveat 'soak evidence is pilot-scoped and diagnostic unless separately replayed' \
                  --caveat 'soak evidence does not grant authority, policy, resource, provenance, or source-gate trust' \
                  --out /var/lib/molten/vm-evidence/prod-soak-run.preserves
                soak_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/prod-soak-run.preserves)
                soak_ref=''${soak_summary#* ref=}
                soak_ref=''${soak_ref%% *}
                molten test nixos-vm run-receipt \
                  --topology /var/lib/molten/vm-evidence/topology.preserves \
                  --node-evidence /var/lib/molten/vm-evidence/node-evidence.preserves \
                  --node-evidence /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                  --scenario phase2-live-control-service-job-restart \
                  --fault-profile executable-vm-faults \
                  --child-ref "$protocol_ref" \
                  --child-ref "$reconcile_ref" \
                  --child-ref "$ack_ref" \
                  --child-ref "$remote_ref" \
                  --child-ref "$job_ref" \
                  --child-ref "$coordination_ref" \
                  --child-ref "$soak_ref" \
                  --child-ref "$vm_fault_validation_ref" \
                  --log /var/lib/molten/vm-evidence/live-control/protocol-gate.txt \
                  --log /var/lib/molten/vm-evidence/live-control/reconcile.txt \
                  --log /var/lib/molten/vm-evidence/live-control/live-loopback.txt \
                  --log /var/lib/molten/vm-evidence/service-job/remote-deliver.txt \
                  --log /var/lib/molten/vm-evidence/service-job/job-ref-execute.txt \
                  --log /var/lib/molten/vm-evidence/service-job/coord-apply.txt \
                  --decision pass \
                  --replay-status non-replayable-vm-observations \
                  --caveat 'vm observations are diagnostic unless separately replayed' \
                  --caveat 'vm evidence does not grant authority or policy trust' \
                  --out /var/lib/molten/vm-evidence/vm-test-run.preserves
              """)
              node_a.succeed("grep -q nixos-vm-topology-v1 /var/lib/molten/vm-evidence/topology.preserves")
              node_a.succeed("grep -q nixos-vm-node-evidence-v1 /var/lib/molten/vm-evidence/node-evidence.preserves")
              node_a.succeed("grep -q nixos-vm-test-run-v1 /var/lib/molten/vm-evidence/vm-test-run.preserves")
              node_a.succeed("grep -q prod-soak-run-v1 /var/lib/molten/vm-evidence/prod-soak-run.preserves")
              node_a.succeed("""
                molten test nixos-vm validate \
                  --topology /var/lib/molten/vm-evidence/topology.preserves \
                  --node-evidence /var/lib/molten/vm-evidence/node-evidence.preserves \
                  --node-evidence /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                  --test-run /var/lib/molten/vm-evidence/vm-test-run.preserves \
                  --prod-soak /var/lib/molten/vm-evidence/prod-soak-run.preserves \
                  --expected-node node_a \
                  --expected-node node_b \
                  --out /var/lib/molten/vm-evidence/vm-evidence-validation.preserves
                molten test nixos-vm manifest \
                  --root /var/lib/molten/vm-evidence \
                  --artifact /var/lib/molten/vm-evidence/topology.preserves \
                  --artifact /var/lib/molten/vm-evidence/node-evidence.preserves \
                  --artifact /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                  --artifact /var/lib/molten/vm-evidence/vm-test-run.preserves \
                  --artifact /var/lib/molten/vm-evidence/prod-soak-run.preserves \
                  --artifact /var/lib/molten/vm-evidence/vm-evidence-validation.preserves \
                  --artifact /var/lib/molten/vm-evidence/vm-faults/network-partition.descriptor.preserves \
                  --artifact /var/lib/molten/vm-evidence/vm-faults/network-partition.receipt.preserves \
                  --artifact /var/lib/molten/vm-evidence/vm-faults/restart.descriptor.preserves \
                  --artifact /var/lib/molten/vm-evidence/vm-faults/restart.receipt.preserves \
                  --artifact /var/lib/molten/vm-evidence/vm-faults/storage.descriptor.preserves \
                  --artifact /var/lib/molten/vm-evidence/vm-faults/storage.receipt.preserves \
                  --artifact /var/lib/molten/vm-evidence/vm-faults/validation.preserves \
                  --log /var/lib/molten/vm-evidence/run.txt \
                  --log /var/lib/molten/vm-evidence/status.txt \
                  --log /var/lib/molten/vm-evidence/live-control/protocol-gate.txt \
                  --log /var/lib/molten/vm-evidence/service-job/job-ref-execute.txt \
                  --log /var/lib/molten/vm-evidence/vm-faults/network-unavailable.log \
                  --log /var/lib/molten/vm-evidence/vm-faults/storage-denial.log \
                  --caveat 'VM canonical receipts are authoritative; preserved logs are diagnostic only' \
                  --out /var/lib/molten/vm-evidence/vm-evidence-manifest.preserves
                grep -q nixos-vm-evidence-validation-v1 /var/lib/molten/vm-evidence/vm-evidence-validation.preserves
                grep -q nixos-vm-evidence-manifest-v1 /var/lib/molten/vm-evidence/vm-evidence-manifest.preserves
                mkdir -p /tmp/molten-vm-output
                cp -R /var/lib/molten/vm-evidence /tmp/molten-vm-output/
              """)
              import os
              out_dir = os.environ["out"]
              os.makedirs(out_dir, exist_ok=True)
              node_a.copy_from_vm("/tmp/molten-vm-output/vm-evidence", os.path.join(out_dir, "vm-evidence"))
            '';
          };

          nextest-config =
            pkgs.runCommand "molten-nextest-config-check"
              {
                nativeBuildInputs = [
                  rustToolchain
                  pkgs.cargo-nextest
                ];
                src = sourceForConfigChecks;
              }
              ''
                set -euo pipefail
                cp -R $src source
                chmod -R u+w source
                cd source
                cargo nextest show-config version --user-config-file none --profile default > default.txt
                cargo nextest show-config version --user-config-file none --profile ci > ci.txt
                cargo nextest show-config version --user-config-file none --profile deterministic > deterministic.txt
                cargo nextest show-config version --user-config-file none --profile exploratory > exploratory.txt
                cargo nextest show-config version --user-config-file none --profile fast-core > fast-core.txt
                cargo nextest show-config version --user-config-file none --profile harness > harness.txt
                cargo nextest show-config version --user-config-file none --profile cli > cli.txt
                cargo nextest show-config version --user-config-file none --profile distributed-simulation > distributed-simulation.txt
                cargo nextest show-config version --user-config-file none --profile vm-platform > vm-platform.txt
                cargo nextest show-config version --user-config-file none --profile dogfood-soak > dogfood-soak.txt
                mkdir -p $out
                cp default.txt ci.txt deterministic.txt exploratory.txt fast-core.txt harness.txt cli.txt distributed-simulation.txt vm-platform.txt dogfood-soak.txt .config/nextest.toml $out/
                printf 'cargo nextest run --profile ci\n' > $out/ci-command.txt
                printf 'cargo nextest run --profile fast-core\n' > $out/fast-core-command.txt
                printf 'cargo nextest run --profile harness\n' > $out/harness-command.txt
                printf 'cargo nextest run --profile cli\n' > $out/cli-command.txt
                printf 'cargo nextest run --profile distributed-simulation\n' > $out/distributed-simulation-command.txt
                printf 'cargo nextest run --profile vm-platform\n' > $out/vm-platform-command.txt
                printf 'cargo nextest run --profile dogfood-soak\n' > $out/dogfood-soak-command.txt
                printf 'target/nextest/ci/junit.xml\n' > $out/ci-junit-path.txt
                printf 'target/nextest/deterministic/junit.xml\n' > $out/deterministic-junit-path.txt
                printf 'target/nextest/harness/junit.xml\n' > $out/harness-junit-path.txt
              '';

          fmt =
            pkgs.runCommand "cargo-fmt-check"
              {
                nativeBuildInputs = [ rustToolchain ];
                src = ./.;
              }
              ''
                cp -R $src source
                chmod -R u+w source
                cd source
                cargo fmt --check
                touch $out
              '';
        };

        apps = {
          nextest-ci = {
            type = "app";
            program = "${nextestCi}/bin/molten-nextest-ci";
            meta = {
              description = "Run Molten's cargo-nextest CI profile";
            };
          };
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            rustToolchain
            pkgs.pkg-config
            pkgs.clang
            pkgs.mold
          ];

          # CLI tools for the embedded languages/protocols used by molten.
          packages = [
            pkgs.steel
            pkgs.nickel
            pkgs.wasmtime
            pkgs.cargo-nextest
            pkgs.cargo-watch
            pkgs.rust-analyzer
            unit2nix.packages.${system}.unit2nix
          ];

          shellHook = ''
            export PATH="$PWD/target/debug:$PATH"
          '';
        };

        formatter = pkgs.nixpkgs-fmt;
      }
    );
}
