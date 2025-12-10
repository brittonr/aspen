{
  description = "mvm-ci";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-25.11";

    crane = {
      url = "github:ipetkov/crane";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  nixConfig = {
    keepOutputs = true;
    max-jobs = "auto";
    builders = "";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    advisory-db,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pname = "mvm-ci";
        lib = nixpkgs.lib;
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };

        rustToolChain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolChain;

        flawless = pkgs.stdenv.mkDerivation {
          pname = "flawless";
          version = "1.0.0-beta.3";
          src = pkgs.fetchurl {
            url = "https://downloads.flawless.dev/1.0.0-beta.3/x64-linux/flawless";
            sha256 = "0p11baphc2s8rjhzn9v2sai52gvbn33y1xlqg2yais6dmf5mj4dm";
          };
          dontUnpack = true;
          nativeBuildInputs = [
            pkgs.autoPatchelfHook
            pkgs.pkg-config
            pkgs.openssl.dev
            pkgs.lld # Linker for WASM targets
            pkgs.stdenv.cc.cc
          ];
          installPhase = ''
            mkdir -p $out/bin
            cp $src $out/bin/flawless
            chmod +x $out/bin/flawless
          '';
        };

        netwatchSrc = pkgs.fetchFromGitHub {
          owner = "n0-computer";
          repo = "net-tools";
          rev = "cd7aba545996781786b8168d49b876f0844ad3d7";
          hash = "sha256-FMuab/Disrd/l9nJFOZ7vQxE2KE98ZLvQvocbZELPPY=";
        };

        netwatchCommon = {
          pname = "netwatch";
          version = "0.12.0";
          src = netwatchSrc;
          cargoToml = "${netwatchSrc}/netwatch/Cargo.toml";
          cargoLock = "${netwatchSrc}/Cargo.lock";
        };

        netwatchCargoArtifacts = craneLib.buildDepsOnly (
          netwatchCommon
          // {
            doCheck = false;
          }
        );

        netwatch = craneLib.buildPackage (
          netwatchCommon
          // {
            cargoArtifacts = netwatchCargoArtifacts;
            cargoExtraArgs = "--bin netwatch";
            doCheck = false;
            postPatch = ''
                          mkdir -p netwatch/src/bin
                          cat <<'RS' > netwatch/src/bin/netwatch.rs
                          use netwatch::netmon::Monitor;
                          use n0_watcher::Watcher as _;

                          #[tokio::main(flavor = "current_thread")]
                          async fn main() -> Result<(), netwatch::netmon::Error> {
                              let monitor = Monitor::new().await?;
                              let mut subscriber = monitor.interface_state();
                              println!("netwatch: initial interface state -> {:#?}", subscriber.get());

                              loop {
                                  tokio::select! {
                                      update = subscriber.updated() => {
                                          match update {
                                              Ok(state) => println!("netwatch: interface state updated -> {:#?}", state),
                                              Err(_) => {
                                                  eprintln!("netwatch: watcher closed, exiting");
                                                  break;
                                              }
                                          }
                                      }
                                      res = tokio::signal::ctrl_c() => {
                                          if let Err(err) = res {
                                              eprintln!("netwatch: ctrl-c handler error: {err}");
                                          }
                                          break;
                                      }
                                  }
                              }
                              Ok(())
                          }
              RS
            '';
          }
        );

        # Use crane's path to properly include vendored openraft
        # Force re-evaluation: updated 2025-12-09
        src = craneLib.path {
          path = ./.;
          # Include everything - vendored openraft needs to be included
          filter = path: type: true;
        };

        basicArgs = {
          inherit src;
          inherit pname;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            git
            pkg-config
            lld # Linker for WASM targets
            protobuf # Protocol Buffers compiler for snix crates
            stdenv.cc.cc
          ];

          buildInputs = with pkgs; [
            openssl
            stdenv.cc.cc.lib # Provides libgcc_s.so.1
          ];

          # Set environment variable required by snix-build at compile time
          SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox}/bin/sh";

          # Set PROTO_ROOT for snix-castore build.rs to find proto files
          # The proto files are vendored in ./vendor/snix
          PROTO_ROOT = "${src}/vendor";
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly basicArgs;

        # Development-specific cargo artifacts that preserve incremental compilation
        # This maintains the Cargo target directory structure for faster rebuilds
        devCargoArtifacts = craneLib.buildDepsOnly (basicArgs
          // {
            # Keep intermediate artifacts for incremental compilation
            doInstallCargoArtifacts = true;
            # Enable incremental compilation in the dependency build
            CARGO_INCREMENTAL = "1";
          });

        # Common arguments can be set here to avoid repeating them later
        commonArgs =
          basicArgs
          // {
            inherit cargoArtifacts;

            nativeBuildInputs =
              basicArgs.nativeBuildInputs
              ++ (with pkgs; [
                autoPatchelfHook
              ]);

            buildInputs =
              basicArgs.buildInputs
              ++ (lib.optionals pkgs.stdenv.buildPlatform.isDarwin (
                with pkgs; [
                  darwin.apple_sdk.frameworks.Security
                ]
              ));
          };

        # Development-specific arguments with incremental compilation enabled
        devArgs =
          commonArgs
          // {
            cargoArtifacts = devCargoArtifacts;
            # Enable incremental compilation
            CARGO_INCREMENTAL = "1";
            # Use more aggressive caching
            CARGO_BUILD_INCREMENTAL = "true";
          };

        mvm-ci = craneLib.buildPackage (
          commonArgs
          // {
            inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
            doCheck = false;
          }
        );

        bins = let
          bin = {name}:
            craneLib.buildPackage (
              commonArgs
              // {
                inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                cargoExtraArgs = "--bin ${name}";
                doCheck = false;
              }
            );
          bins = builtins.listToAttrs (
            map ({name, ...} @ package: lib.nameValuePair name (bin package)) [
              {
                name = "cibtool";
              }
              {
                name = "cib";
              }
              {
                name = "synthetic-events";
              }
              {
                name = "mvm-ci";
              }
              {
                name = "worker";
              }
              {
                name = "aspen-node";
              }
              {
                name = "aspen-tui";
              }
            ]
          );
        in
          bins
          // rec {
            default = mvm-ci;
            ci-broker = pkgs.buildEnv {
              name = "mvm-ci";
              paths = with bins; [
                cibtool
                cib
                synthetic-events
              ];
            };

            # Development builds with incremental compilation enabled
            # Use these for faster iteration during development
            dev-aspen-node = craneLib.buildPackage (
              devArgs
              // {
                inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                cargoExtraArgs = "--bin aspen-node";
                doCheck = false;
              }
            );

            dev-aspen-tui = craneLib.buildPackage (
              devArgs
              // {
                inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                cargoExtraArgs = "--bin aspen-tui";
                doCheck = false;
              }
            );

            # Convenience alias for the most commonly used dev build
            dev = dev-aspen-node;
          };
      in {
        # Formatter
        formatter = pkgs.alejandra;

        # Set of checks that are run: `nix flake check`
        checks = {
          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          clippy = craneLib.cargoClippy (
            commonArgs
            // {
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            }
          );

          doc = craneLib.cargoDoc commonArgs;
          fmt = craneLib.cargoFmt basicArgs;
          deny = craneLib.cargoDeny commonArgs;

          audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Run tests with cargo-nextest
          nextest = craneLib.cargoNextest (
            commonArgs
            // {
              # We skip the test since it uses the underlying .git directory,
              # which is not available in the Nix sandbox.
              # In any case, this test is slow and we expect it to be tested
              # before merges (and it can be tested in the devShell)
              cargoNextestExtraArgs = "-- --skip acceptance_criteria_for_upgrades";
              partitions = 1;
              partitionType = "count";
              nativeBuildInputs = [
                pkgs.bash
                pkgs.git
                pkgs.jq
                pkgs.sqlite
              ];

              # Ensure dev is used since we rely on env variables being
              # set in tests.
              env.CARGO_PROFILE = "dev";

              # Collect simulation artifacts if tests fail
              postInstall = ''
                if [ -d docs/simulations ]; then
                  mkdir -p $out/simulations
                  cp -r docs/simulations/*.json $out/simulations/ 2>/dev/null || true
                  if [ -n "$(ls -A $out/simulations 2>/dev/null)" ]; then
                    echo "Simulation artifacts collected in $out/simulations"
                  fi
                fi
              '';
            }
          );

          # Integration tests with flawless server
          # Run with: nix run .#integration-tests
          # Or check with: nix flake check -L
          integration-tests = pkgs.stdenv.mkDerivation {
            name = "mvm-ci-integration-tests";
            src = ./.;

            nativeBuildInputs = [
              rustToolChain
              flawless
              pkgs.bash
              pkgs.curl
              pkgs.git
              pkgs.jq
              pkgs.sqlite
              pkgs.pkg-config
              pkgs.openssl.dev
              pkgs.protobuf
            ];

            buildInputs = [
              pkgs.openssl
            ];

            # Required environment variables
            env.SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox}/bin/sh";
            env.RUST_SRC_PATH = "${rustToolChain}/lib/rustlib/src/rust/library";

            # Disable sandbox for network access (flawless server needs it)
            __noChroot = true;

            buildPhase = ''
              export HOME=$TMPDIR
              export CARGO_HOME=$TMPDIR/.cargo
              mkdir -p $CARGO_HOME

              echo "Building project for tests..."
              cargo build --tests
            '';

            checkPhase = ''
              echo "Running integration test script..."
              bash ${./scripts/run-integration-tests.sh}
            '';

            installPhase = ''
              mkdir -p $out
              echo "Integration tests passed" > $out/result
            '';

            doCheck = true;
          };
        };

        packages.default = bins.aspen-node;
        packages.aspen-node = bins.aspen-node;
        packages.aspen-tui = bins.aspen-tui;
        packages.netwatch = netwatch;

        # Docker image for cluster testing (using streamLayeredImage for better caching)
        packages.dockerImage = pkgs.dockerTools.streamLayeredImage {
          name = "mvm-ci-cluster";
          tag = "latest";

          contents = [
            mvm-ci
            flawless
            pkgs.bash
            pkgs.coreutils
            pkgs.gettext # for envsubst
            pkgs.cacert
            # Add entrypoint scripts and config template
            (pkgs.runCommand "mvm-ci-extras" {} ''
              mkdir -p $out/bin $out/etc
              cp ${./docker-entrypoint.sh} $out/bin/docker-entrypoint.sh
              cp ${./worker-entrypoint.sh} $out/bin/worker-entrypoint.sh
              chmod +x $out/bin/docker-entrypoint.sh
              chmod +x $out/bin/worker-entrypoint.sh
            '')
          ];

          config = {
            Cmd = [
              "${pkgs.bash}/bin/sh"
              "/bin/docker-entrypoint.sh"
            ];
            Env = [
              "PATH=/bin"
              "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            ];
            ExposedPorts = {
              "3020/tcp" = {};
              "9000/tcp" = {};
              "9001/tcp" = {};
              "27288/tcp" = {};
            };
            WorkingDir = "/";
          };
        };

        apps.cibtool = flake-utils.lib.mkApp {
          drv = self.bins.${system}.cibtool;
        };

        apps.cib = flake-utils.lib.mkApp {
          drv = self.bins.${system}.cib;
        };

        apps.synthetic-events = flake-utils.lib.mkApp {
          drv = self.bins.${system}.synthetic-events;
        };

        apps.aspen-node = flake-utils.lib.mkApp {
          drv = bins.aspen-node;
          exePath = "/bin/aspen-node";
        };

        apps.aspen-tui = flake-utils.lib.mkApp {
          drv = bins.aspen-tui;
          exePath = "/bin/aspen-tui";
        };

        # 3-node cluster launcher
        # Usage: nix run .#cluster
        # Environment variables:
        #   ASPEN_NODE_COUNT  - Number of nodes (default: 3)
        #   ASPEN_BASE_HTTP   - Base HTTP port (default: 21001)
        #   ASPEN_STORAGE     - Storage backend: inmemory, sqlite, redb (default: inmemory)
        apps.cluster = {
          type = "app";
          program = "${pkgs.writeShellScript "aspen-cluster" ''
            export PATH="${pkgs.lib.makeBinPath [
              bins.aspen-node
              pkgs.bash
              pkgs.coreutils
              pkgs.curl
              pkgs.netcat
              pkgs.gnugrep
            ]}:$PATH"
            export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
            exec ${./scripts/cluster.sh} "$@"
          ''}";
        };

        # Launch cluster with TUI in separate Kitty window
        # Usage: nix run .#cluster-with-tui
        # Requires: Running from within Kitty terminal
        apps.cluster-with-tui = {
          type = "app";
          program = "${pkgs.writeShellScript "aspen-cluster-with-tui" ''
            # Check if running in Kitty
            if [ -z "''${KITTY_WINDOW_ID:-}" ]; then
              echo "Error: This must be run from within Kitty terminal"
              echo ""
              echo "Alternative: You can run these separately:"
              echo "  1. In one terminal: nix run .#cluster"
              echo "  2. In another: nix run .#aspen-tui -- --nodes http://127.0.0.1:21001"
              exit 1
            fi

            export PATH="${pkgs.lib.makeBinPath [
              bins.aspen-node
              bins.aspen-tui
              pkgs.bash
              pkgs.coreutils
              pkgs.curl
              pkgs.netcat
              pkgs.gnugrep
              pkgs.jq
              pkgs.kitty
            ]}:$PATH"
            export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
            exec ${./scripts/cluster-with-tui.sh} ${./scripts/cluster.sh}
          ''}";
        };

        # Launch cluster with TUI in tmux session (works anywhere)
        # Usage: nix run .#cluster-tui
        apps.cluster-tui = {
          type = "app";
          program = "${pkgs.writeShellScript "aspen-cluster-tui-tmux" ''
            export PATH="${pkgs.lib.makeBinPath [
              bins.aspen-node
              bins.aspen-tui
              pkgs.bash
              pkgs.coreutils
              pkgs.curl
              pkgs.netcat
              pkgs.gnugrep
              pkgs.jq
              pkgs.tmux
            ]}:$PATH"
            export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
            exec ${./scripts/cluster-with-tui-tmux.sh}
          ''}";
        };

        apps.default = self.apps.${system}.aspen-node;

        # Integration tests app - starts flawless server and runs tests
        apps.integration-tests = {
          type = "app";
          program = "${pkgs.writeShellScript "run-integration-tests" ''
            export PATH="${
              pkgs.lib.makeBinPath [
                rustToolChain
                flawless
                pkgs.bash
                pkgs.curl
                pkgs.coreutils
                pkgs.cargo-nextest
              ]
            }:$PATH"
            export SNIX_BUILD_SANDBOX_SHELL="${pkgs.busybox}/bin/sh"

            echo "Running integration tests with flawless server..."
            exec ${./scripts/run-integration-tests.sh}
          ''}";
        };

        devShells.default = craneLib.devShell {
          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = with pkgs; [
            flawless
            netwatch
            litefs # Transparent SQLite replication via FUSE filesystem
            bash
            coreutils
            cargo-watch
            cargo-nextest
            git
            jq
            ripgrep
            rust-analyzer
            sqlite
            pkg-config
            openssl.dev
            codex
            lld # Linker for WASM targets
            protobuf # Protocol Buffers compiler for snix crates
            # Pre-commit and quality tools
            pre-commit
            shellcheck
            nodePackages.markdownlint-cli
            # Optional: sccache for additional caching
            sccache
          ];

          env.RUST_SRC_PATH = "${rustToolChain}/lib/rustlib/src/rust/library";
          env.SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox}/bin/sh";

          # Incremental compilation settings for faster rebuilds
          env.CARGO_INCREMENTAL = "1";
          env.CARGO_BUILD_INCREMENTAL = "true";

          # Optional: Use sccache if available
          # Uncomment to enable sccache globally in dev shell
          # env.RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";

          # Configure cargo to use a shared target directory for better caching
          # This prevents duplicate builds when switching between nix develop and direct cargo commands
          env.CARGO_TARGET_DIR = "target";

          # Enable cargo's new resolver for better dependency resolution
          env.CARGO_RESOLVER = "2";

          shellHook = ''
            echo "🚀 Incremental builds enabled for faster iteration"
            echo "   - Use 'nix build .#dev-aspen-node' for incremental Nix builds"
            echo "   - Use 'cargo build' in this shell for local incremental compilation"
            echo "   - Optional: Run 'export RUSTC_WRAPPER=${pkgs.sccache}/bin/sccache' to enable sccache"
            echo ""
            echo "💡 Tip: Clean up with 'cargo clean' periodically to prevent disk bloat"
          '';
        };
      }
    );
}
