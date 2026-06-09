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
    basalt-src = {
      url = "path:/home/brittonr/.cargo/git/checkouts/basalt-d217f0a83bebd193/005e149";
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
    trellis-src = {
      url = "path:/home/brittonr/.cargo/git/checkouts/trellis-71b30c19277df8df/68ac582";
      flake = false;
    };
    ucan-src = {
      url = "path:/home/brittonr/.cargo/git/checkouts/ucan-9abe9593165792e6/ad61b53";
      flake = false;
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, unit2nix, rust-overlay, flake-utils, basalt-src, cairn-src, octet-src, trellis-src, ucan-src, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgsBase = import nixpkgs {
          localSystem = system;
          overlays = [ (import rust-overlay) ];
        };

        localGitSources = {
          "ssh://git@github.com/OnixResearch/basalt.git#005e1496a4a1be477ba84008ecbcdf8793a236c6" = basalt-src;
          "ssh://git@github.com/OnixResearch/cairn.git#3b4c280b893f2709aebea21fc51a4f9eeba3fe3b" = cairn-src;
          "ssh://git@github.com/OnixResearch/octet.git#9b6a2065ef9e8e363d81299cf59d74f885926215" = octet-src;
          "ssh://git@github.com/OnixResearch/trellis.git#68ac5824f0ef664e4bedeb8ea92ee938b9e00da0" = trellis-src;
          "ssh://git@github.com/OnixResearch/ucan.git#ad61b53e89fa45f9bf7d313ce14c45de645bf53d" = ucan-src;
        };

        pkgs = pkgsBase;
        unit2nixPkgsBase = pkgsBase.extend (final: prev: {
          fetchgit = (prev.lib.makeOverridable (args:
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
          )) // {
            inherit (prev.fetchgit) getRevWithTag;
          };
        });

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rustToolchainCompat = rustToolchain // {
          unwrapped = rustToolchain // {
            configureFlags = [ "--target=${pkgs.stdenv.hostPlatform.rust.rustcTarget}" ];
          };
        };
        unit2nixPkgs = unit2nixPkgsBase.extend (final: prev: {
          # unit2nix auto mode forwards the custom toolchain to Cargo's
          # unit-graph generation, but this pinned revision does not forward it
          # to the clippy wrapper. Make pkgs.clippy/pkgs.rustc match the
          # dependency compiler so Nix flake checks do not mix rustc metadata.
          clippy = rustToolchain;
          rustc = rustToolchainCompat;
        });

        ws = unit2nix.lib.${system}.buildFromUnitGraphAuto {
          pkgs = unit2nixPkgs;
          inherit rustToolchain;
          src = ./.;
          workspace = true;
          noLocked = true;
          clippyArgs = [ "-D" "warnings" ];
          buildRustCrateForPkgs = pkgs: pkgs.buildRustCrate.override {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };
          extraCrateOverrides = {
            # nickel-lang-core declares links="nix", but the Nix FFI is behind
            # the disabled nix-experimental feature in this workspace.
            nickel-lang-core = attrs: { };
            # verus_prettyplease declares links="prettyplease-verus02" but
            # vendors its implementation; no native libraries are required.
            verus_prettyplease = attrs: { };
          };
        };

        moltenPkg = ws.workspaceMembers."molten".build;
        moltenTestBinaries = (ws.test.workspaceMembers."molten".build).override { buildTests = true; };
        targetTriple = pkgs.stdenv.hostPlatform.rust.rustcTarget;
        rustLibDir = "${rustToolchain}/lib/rustlib/${targetTriple}/lib";
        nextestCi = pkgs.writeShellApplication {
          name = "molten-nextest-ci";
          runtimeInputs = [ rustToolchain pkgs.cargo-nextest ];
          text = ''
            exec cargo nextest run --profile ci "$@"
          '';
        };
        sourceForConfigChecks = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            let
              base = baseNameOf path;
            in
              !(base == "target" || base == ".direnv" || base == ".git");
        };
      in
      {
        packages = {
          default = moltenPkg;
          molten = moltenPkg;
          all = ws.allWorkspaceMembers;
        };

        checks = rec {
          # The hermetic nextest check supplies binary metadata for CLI tests
          # using CARGO_BIN_EXE_molten; the raw unit2nix libtest runner does not.
          molten = nextest;
          clippy = ws.clippy.allWorkspaceMembers;

          nextest = pkgs.runCommand "molten-nextest"
            {
              nativeBuildInputs = [ rustToolchain pkgs.cargo-nextest pkgs.perl ];
              src = sourceForConfigChecks;
              testBinaries = moltenTestBinaries;
              inherit targetTriple rustLibDir;
            } ''
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
            if [ -f target/nextest/junit.xml ]; then
              cp target/nextest/junit.xml "$out"/
            fi
          '';

          dogfood-local-node = pkgs.runCommand "molten-dogfood-local-node"
            {
              nativeBuildInputs = [ moltenPkg pkgs.gnugrep ];
              src = sourceForConfigChecks;
              nextestCheck = nextest;
            } ''
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
              > dogfood-summary.txt
            grep -q 'decision=pass' dogfood-summary.txt
            grep -q 'dogfood-report-v1' dogfood-report.preserves
            grep -q 'release-gate-receipt-v1' release-gate.preserves
            mkdir -p "$out"
            cp dogfood-summary.txt dogfood-report.preserves release-gate.preserves "$out"/
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
              --signed-key local-release-key \
              --signed-signer local-release-signer \
              --signed-member "$out/dogfood-report.signed.preserves" \
              --signed-member "$out/release-gate.signed.preserves" \
              --signed-member "$out/nix-dogfood-evidence.signed.preserves" \
              --signed-member "$out/nix-dogfood-verify.signed.preserves" \
              | tee "$out/release-evidence-bundle-verify.txt"
            grep -q 'decision=pass' "$out/release-evidence-bundle-verify.txt"
          '';

          nextest-config = pkgs.runCommand "molten-nextest-config-check"
            {
              nativeBuildInputs = [ rustToolchain pkgs.cargo-nextest ];
              src = sourceForConfigChecks;
            } ''
            cp -R $src source
            chmod -R u+w source
            cd source
            cargo nextest show-config version --user-config-file none --profile default > default.txt
            cargo nextest show-config version --user-config-file none --profile ci > ci.txt
            cargo nextest show-config version --user-config-file none --profile deterministic > deterministic.txt
            cargo nextest show-config version --user-config-file none --profile exploratory > exploratory.txt
            mkdir -p $out
            cp default.txt ci.txt deterministic.txt exploratory.txt $out/
            printf 'cargo nextest run --profile ci\n' > $out/ci-command.txt
          '';

          fmt = pkgs.runCommand "cargo-fmt-check"
            {
              nativeBuildInputs = [ rustToolchain ];
              src = ./.;
            } ''
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
      });
}

