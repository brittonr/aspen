{
  description = "Aspen - Foundational orchestration layer for distributed systems";

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

    # SNIX - Nix store implementation in Rust
    # Used for content-addressed storage and Nix binary cache integration
    snix-src = {
      url = "git+https://git.snix.dev/snix/snix.git?rev=8fe3bade2013befd5ca98aa42224fa2a23551559";
      flake = false;
    };

    # MicroVM.nix - Declarative NixOS microVMs for isolated dogfood testing
    # Uses Cloud Hypervisor for fast boot times (~125ms) with full kernel isolation
    microvm = {
      url = "github:astro/microvm.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    keepOutputs = true;
    max-jobs = "auto";
    builders = "";
    # Binary caches
    extra-substituters = [
      "https://cache.nixos.org"
      # microvm.nix provides pre-built Cloud Hypervisor kernels and VM components
      "https://microvm.cachix.org"
      # TODO: Add your Harmonia/Attic URL for project-specific cache, e.g.:
      # "https://cache.yourserver.com"
    ];
    extra-trusted-public-keys = [
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      "microvm.cachix.org-1:oXnBc6hRE3eX5rSYdRyMYXnfzcCxC7yKPTbZXALsqys="
      # TODO: Add your cache public key, e.g.:
      # "cache.yourserver.com-1:AAAA..."
    ];
    # Network reliability
    connect-timeout = 30;
    download-attempts = 3;
    fallback = true;
    # Automatic garbage collection
    min-free = 5368709120; # 5GB - trigger GC when less than this free
    max-free = 10737418240; # 10GB - stop GC when this much free
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    advisory-db,
    rust-overlay,
    snix-src,
    microvm,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pname = "aspen";
        lib = nixpkgs.lib;
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };

        rustToolChain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolChain;

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
        rawSrc = craneLib.path {
          path = ./.;
          # Include everything - vendored openraft needs to be included
          filter = path: type: true;
        };

        # The .cargo/config.toml has [patch] entries pointing to ../snix/snix/*
        # which won't exist in the Nix sandbox. We need to provide snix at the
        # expected relative path. Since we can't modify paths outside the source
        # derivation, we use a setup hook to copy snix to the build directory
        # before cargo runs.
        #
        # Strategy: The source itself is clean, but we use preBuild to set up
        # the ../snix directory relative to the unpacked source in /build/.
        src = rawSrc;

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
            clang # Required by .cargo/config.toml linker setting
            mold # Fast linker used with clang
            autoPatchelfHook # Patch build scripts to find shared libs
          ];

          buildInputs = with pkgs; [
            openssl
            zlib # Required by hyperlight-host build script
            stdenv.cc.cc.lib # Provides libgcc_s.so.1
          ];

          # Ensure libraries are available for build scripts
          LD_LIBRARY_PATH = lib.makeLibraryPath [pkgs.zlib pkgs.stdenv.cc.cc.lib];

          # Set environment variable required by snix-build at compile time
          SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox}/bin/sh";

          # Set PROTO_ROOT for snix-castore build.rs to find proto files
          # Points to the snix source fetched as a flake input
          PROTO_ROOT = "${snix-src}";

          # Copy snix source to /build/snix so the [patch] paths in .cargo/config.toml resolve.
          # The config has: path = "../snix/snix/castore" which resolves to /build/snix/snix/castore
          # when cargo runs from /build/<source-root>/
          preBuild = ''
            # Set up snix at the expected location for cargo patches
            # The .cargo/config.toml has [patch] entries pointing to ../snix/snix/*
            if [ ! -d /build/snix ]; then
              echo "Setting up snix at /build/snix for cargo patches..."
              cp -r ${snix-src} /build/snix
              chmod -R u+w /build/snix
            fi
          '';
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly basicArgs;

        # Development-specific cargo artifacts that preserve incremental compilation
        # This maintains the Cargo target directory structure for faster rebuilds
        devCargoArtifacts = craneLib.buildDepsOnly (
          basicArgs
          // {
            # Keep intermediate artifacts for incremental compilation
            doInstallCargoArtifacts = true;
            # Enable incremental compilation in the dependency build
            CARGO_INCREMENTAL = "1";
          }
        );

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

        # Build the main package
        aspen = craneLib.buildPackage (
          commonArgs
          // {
            inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
            doCheck = false;
          }
        );

        # Helper script for setting up VM test environment
        vm-test-setup = pkgs.writeShellScriptBin "aspen-vm-setup" ''
          #!/usr/bin/env bash
          set -e

          echo "Setting up Cloud Hypervisor test environment..."

          # Check for KVM support
          if [ ! -e /dev/kvm ]; then
            echo "ERROR: KVM not available. Please ensure KVM is enabled."
            exit 1
          fi

          # Create network bridge if it doesn't exist
          if ! ip link show aspen-br0 &>/dev/null; then
            echo "Creating network bridge aspen-br0..."
            sudo ip link add aspen-br0 type bridge
            sudo ip addr add 10.100.0.1/24 dev aspen-br0
            sudo ip link set aspen-br0 up
            echo "Bridge created successfully."
          else
            echo "Bridge aspen-br0 already exists."
          fi

          # Create TAP devices for testing (up to 10)
          for i in {0..9}; do
            if ! ip link show aspen-tap$i &>/dev/null; then
              echo "Creating TAP device aspen-tap$i..."
              sudo ip tuntap add aspen-tap$i mode tap
              sudo ip link set aspen-tap$i master aspen-br0
              sudo ip link set aspen-tap$i up
            fi
          done

          echo "Setup complete! You can now run VM tests."
          echo ""
          echo "Quick start:"
          echo "  cloud-hypervisor --api-socket /tmp/ch.sock"
          echo ""
          echo "Or use the aspen-vm-run helper to launch a test VM."
        '';

        # Helper script for running a test VM
        vm-test-run = pkgs.writeShellScriptBin "aspen-vm-run" ''
          #!/usr/bin/env bash
          set -e

          NODE_ID=''${1:-1}

          # Default paths (can be overridden with env vars)
          KERNEL=''${CH_KERNEL:-${pkgs.linuxPackages.kernel}/bzImage}
          INITRD=''${CH_INITRD:-}
          DISK=''${CH_DISK:-/tmp/aspen-node-$NODE_ID.img}

          # Create a simple disk if it doesn't exist
          if [ ! -f "$DISK" ]; then
            echo "Creating disk image at $DISK..."
            qemu-img create -f raw "$DISK" 1G
          fi

          # Calculate TAP device and MAC address based on node ID
          TAP_NUM=$((NODE_ID - 1))
          TAP_NAME="aspen-tap$TAP_NUM"
          MAC=$(printf "52:54:00:00:00:%02x" $NODE_ID)

          # Cloud Hypervisor creates its own TAP devices
          # If a TAP with the same name exists, it will fail
          # So we either remove it first or use a unique name

          # Option 1: Remove existing TAP if it exists (requires sudo)
          if ip link show "$TAP_NAME" &>/dev/null; then
            echo "Note: TAP device $TAP_NAME already exists. Removing it..."
            sudo ip link del "$TAP_NAME" 2>/dev/null || true
          fi

          echo "Launching Cloud Hypervisor VM for node $NODE_ID..."
          echo "  Kernel: $KERNEL"
          echo "  Disk: $DISK"
          echo "  Network: TAP $TAP_NAME (will be created), MAC $MAC"
          echo ""
          echo "Starting VM (requires sudo for TAP creation)..."
          echo "Press Ctrl+C to stop the VM."
          echo ""

          # Cloud Hypervisor will create the TAP device
          sudo cloud-hypervisor \
            --kernel "$KERNEL" \
            --disk path="$DISK" \
            --cmdline "console=hvc0 root=/dev/vda1 rw init=/bin/sh" \
            --cpus boot=2 \
            --memory size=512M \
            --net "tap=$TAP_NAME,mac=$MAC,ip=10.100.0.$((NODE_ID+1))/24" \
            --serial tty \
            --console off \
            --api-socket /tmp/ch-node-$NODE_ID.sock
        '';

        bins = let
          bin = {
            name,
            features ? [],
          }:
            craneLib.buildPackage (
              commonArgs
              // {
                inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                cargoExtraArgs =
                  "--bin ${name}"
                  + lib.optionalString (features != []) " --features ${lib.concatStringsSep "," features}";
                doCheck = false;
              }
            );

          # Build aspen-tui from its own crate
          aspen-tui-crate = craneLib.buildPackage (
            commonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-tui/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-tui --bin aspen-tui";
              doCheck = false;
            }
          );

          # Build aspen-cli from its own crate
          aspen-cli-crate = craneLib.buildPackage (
            commonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli";
              doCheck = false;
            }
          );

          # Build aspen-ci-agent from its own crate
          aspen-ci-agent-crate = craneLib.buildPackage (
            commonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-ci-agent/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-ci-agent --bin aspen-ci-agent";
              doCheck = false;
            }
          );

          bins =
            builtins.listToAttrs (
              map ({name, ...} @ package: lib.nameValuePair name (bin package)) [
                {name = "aspen-node";}
                {
                  name = "git-remote-aspen";
                  features = ["git-bridge"];
                }
              ]
            )
            // {
              aspen-tui = aspen-tui-crate;
              aspen-cli = aspen-cli-crate;
              aspen-ci-agent = aspen-ci-agent-crate;
            };
        in
          bins
          // rec {
            default = bins.aspen-node;

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
                inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-tui/Cargo.toml;}) pname version;
                cargoExtraArgs = "--package aspen-tui --bin aspen-tui";
                doCheck = false;
              }
            );

            # Convenience alias for the most commonly used dev build
            dev = dev-aspen-node;
          };
      in
        {
          # Formatter
          formatter = pkgs.alejandra;

          # Set of checks that are run: `nix flake check`
          checks =
            {
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

              # Custom fmt check using nightly rustfmt to support unstable features
              # in rustfmt.toml (imports_granularity, overflow_delimited_expr, etc.)
              fmt =
                pkgs.runCommand "aspen-fmt-check" {
                  nativeBuildInputs = [pkgs.rust-bin.nightly.latest.rustfmt pkgs.findutils];
                } ''
                  cd ${src}
                  find . -name '*.rs' \
                    -not -path './target/*' \
                    -not -path './openraft/*' \
                    -not -path './vendor/*' \
                    -exec rustfmt --edition 2024 --check {} +
                  touch $out
                '';

              deny = craneLib.cargoDeny commonArgs;

              audit = craneLib.cargoAudit {
                inherit src advisory-db;
              };
            }
            // {
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
            };

          # Base apps available on all systems
          apps = {
            aspen-node = flake-utils.lib.mkApp {
              drv = bins.aspen-node;
              exePath = "/bin/aspen-node";
            };

            aspen-tui = flake-utils.lib.mkApp {
              drv = bins.aspen-tui;
              exePath = "/bin/aspen-tui";
            };

            aspen-cli = flake-utils.lib.mkApp {
              drv = bins.aspen-cli;
              exePath = "/bin/aspen-cli";
            };

            # 3-node cluster launcher
            # Usage: nix run .#cluster
            # Environment variables:
            #   ASPEN_NODE_COUNT  - Number of nodes (default: 3)
            #   ASPEN_BASE_HTTP   - Base HTTP port (default: 21001)
            #   ASPEN_STORAGE     - Storage backend: inmemory, sqlite, redb (default: sqlite)
            cluster = let
              # Bundle scripts directory so lib/cluster-common.sh is available
              scriptsDir = pkgs.runCommand "aspen-scripts" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +w $out
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-cluster" ''
                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-node
                    bins.aspen-cli
                    pkgs.bash
                    pkgs.coreutils
                    pkgs.curl
                    pkgs.netcat
                    pkgs.gnugrep
                    pkgs.gnused
                  ]
                }:$PATH"
                export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                exec ${scriptsDir}/cluster.sh "$@"
              ''}";
            };

            # Kitty terminal cluster (N nodes + TUI in tabs)
            # Usage: nix run .#kitty-cluster
            # Opens a kitty window with node tabs + TUI tab
            kitty-cluster = let
              # Bundle scripts directory so lib/cluster-common.sh is available
              scriptsDir = pkgs.runCommand "aspen-scripts" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +w $out
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-kitty-cluster" ''
                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-node
                    bins.aspen-cli
                    bins.aspen-tui
                    pkgs.kitty
                    pkgs.coreutils
                    pkgs.gnugrep
                    pkgs.gawk
                    pkgs.gnused
                  ]
                }:$PATH"
                export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                export ASPEN_TUI_BIN="${bins.aspen-tui}/bin/aspen-tui"
                exec ${scriptsDir}/kitty-cluster.sh "$@"
              ''}";
            };

            # CLI test suite - tests all CLI commands against a running cluster
            # Usage: nix run .#cli-test -- --ticket <ticket>
            #        ASPEN_TICKET=<ticket> nix run .#cli-test
            # Options:
            #   --skip-slow     Skip slow tests (blob, job)
            #   --category X    Run only specific category (cluster, kv, counter, etc.)
            #   --verbose       Show full command output
            #   --json          Output results as JSON
            cli-test = let
              scriptsDir = pkgs.runCommand "aspen-scripts" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +w $out
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-cli-test" ''
                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-cli
                    pkgs.bash
                    pkgs.coreutils
                    pkgs.gnugrep
                    pkgs.gnused
                  ]
                }:$PATH"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                exec ${scriptsDir}/cli-test.sh "$@"
              ''}";
            };

            # Hooks CLI integration test - tests all hooks commands
            # Usage: nix run .#kitty-hooks-test
            #        nix run .#kitty-hooks-test -- --ticket <ticket>
            # Options:
            #   --ticket <t>    Use existing cluster
            #   --node-count N  Number of nodes (default: 3)
            #   --keep-cluster  Don't stop cluster after tests
            #   --verbose       Show full command output
            #   --json          Output results as JSON
            kitty-hooks-test = let
              scriptsDir = pkgs.runCommand "aspen-scripts" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +x $out/
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-kitty-hooks-test" ''
                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-node
                    bins.aspen-cli
                    pkgs.coreutils
                    pkgs.gnugrep
                    pkgs.gnused
                    pkgs.gawk
                  ]
                }:$PATH"
                export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                exec ${scriptsDir}/kitty-hooks-test.sh "$@"
              ''}";
            };

            # Secrets CLI integration test - tests all secrets commands (KV, Transit, PKI)
            # Usage: nix run .#kitty-secrets-test
            #        nix run .#kitty-secrets-test -- --ticket <ticket>
            # Options:
            #   --ticket <t>      Use existing cluster
            #   --node-count N    Number of nodes (default: 3)
            #   --category <name> Run only specific category (kv, transit, pki, workflow)
            #   --keep-cluster    Don't stop cluster after tests
            #   --verbose         Show full command output
            #   --json            Output results as JSON
            kitty-secrets-test = let
              scriptsDir = pkgs.runCommand "aspen-scripts" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +x $out/
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-kitty-secrets-test" ''
                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-node
                    bins.aspen-cli
                    pkgs.coreutils
                    pkgs.gnugrep
                    pkgs.gnused
                    pkgs.gawk
                  ]
                }:$PATH"
                export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                exec ${scriptsDir}/kitty-secrets-test.sh "$@"
              ''}";
            };

            # Comprehensive CLI integration test - tests ALL CLI commands
            # Usage: nix run .#kitty-cli-test
            #        nix run .#kitty-cli-test -- --ticket <ticket>
            # Options:
            #   --ticket <t>      Use existing cluster (skip startup)
            #   --node-count N    Number of nodes (default: 3)
            #   --skip-slow       Skip slow tests (blob, job)
            #   --category <name> Run only specific category
            #   --keep-cluster    Don't stop cluster after tests
            #   --verbose         Show full command output
            #   --json            Output results as JSON
            kitty-cli-test = let
              scriptsDir = pkgs.runCommand "aspen-scripts" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +x $out/
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-kitty-cli-test" ''
                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-node
                    bins.aspen-cli
                    pkgs.coreutils
                    pkgs.gnugrep
                    pkgs.gnused
                    pkgs.gawk
                    pkgs.procps
                  ]
                }:$PATH"
                export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                exec ${scriptsDir}/kitty-cli-test.sh "$@"
              ''}";
            };

            # Git/Forge integration tests in kitty terminal
            # Usage: nix run .#kitty-forge-test
            kitty-forge-test = let
              scriptsDir = pkgs.runCommand "kitty-forge-test-scripts" {} ''
                mkdir -p $out/lib
                cp ${./scripts/kitty-forge-test.sh} $out/kitty-forge-test.sh
                cp -r ${./scripts/lib}/* $out/lib/ 2>/dev/null || true
                chmod -R +x $out/
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-kitty-forge-test" ''
                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-node
                    bins.aspen-cli
                    pkgs.coreutils
                    pkgs.gnugrep
                    pkgs.gnused
                    pkgs.gawk
                    pkgs.procps
                  ]
                }:$PATH"
                export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                exec ${scriptsDir}/kitty-forge-test.sh "$@"
              ''}";
            };

            # Issue/Patch collaboration tests in kitty terminal
            # Usage: nix run .#kitty-collab-test
            kitty-collab-test = let
              scriptsDir = pkgs.runCommand "kitty-collab-test-scripts" {} ''
                mkdir -p $out/lib
                cp ${./scripts/kitty-collab-test.sh} $out/kitty-collab-test.sh
                cp -r ${./scripts/lib}/* $out/lib/ 2>/dev/null || true
                chmod -R +x $out/
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-kitty-collab-test" ''
                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-node
                    bins.aspen-cli
                    pkgs.coreutils
                    pkgs.gnugrep
                    pkgs.gnused
                    pkgs.gawk
                    pkgs.procps
                  ]
                }:$PATH"
                export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                exec ${scriptsDir}/kitty-collab-test.sh "$@"
              ''}";
            };

            # Default: single development node with sensible defaults
            # Usage: nix run
            # This starts a single-node cluster ready for experimentation.
            # For production, use: nix run .#aspen-node -- --node-id 1 --cookie <secret>
            # For multi-node cluster: nix run .#cluster
            default = {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-dev" ''
                set -e

                # Generate a unique data directory for this session
                DATA_DIR="''${ASPEN_DATA_DIR:-/tmp/aspen-dev-$$}"
                mkdir -p "$DATA_DIR"

                echo "Starting Aspen development node..."
                echo ""
                echo "  Node ID:    1"
                echo "  Cookie:     dev-cookie-$USER"
                echo "  Data dir:   $DATA_DIR"
                echo ""
                echo "This is a single-node development cluster."
                echo "For production, use: nix run .#aspen-node -- --node-id 1 --cookie <secret>"
                echo "For multi-node:      nix run .#cluster"
                echo ""

                exec ${bins.aspen-node}/bin/aspen-node \
                  --node-id 1 \
                  --cookie "dev-cookie-$USER" \
                  --data-dir "$DATA_DIR" \
                  "$@"
              ''}";
            };

            # Fuzzing commands - run with nix run .#fuzz, .#fuzz-quick, .#fuzz-intensive
            # NOTE: Must be run from the project root directory
            fuzz = {
              type = "app";
              program = "${pkgs.writeShellScript "fuzz-all" ''
                set -e

                if [ ! -f "fuzz/Cargo.toml" ]; then
                  echo "Error: Must be run from the aspen project root directory"
                  exit 1
                fi

                echo "Starting parallel fuzzing campaign on high-risk targets (1 hour each)..."
                echo ""

                mkdir -p fuzz/dictionaries

                # Enter the fuzzing devShell environment
                nix develop .#fuzz --command bash -c '
                  # Function to run fuzzer and capture dictionary
                  run_and_capture() {
                    local target=$1
                    local output_file=$(mktemp)

                    cargo fuzz run "$target" -- -max_total_time=3600 2>&1 | tee "$output_file"

                    # Extract and save recommended dictionary
                    if grep -q "Recommended dictionary" "$output_file"; then
                      local dict_file="fuzz/dictionaries/$target-auto.dict"
                      echo "# Auto-generated dictionary from fuzzing run on $(date -I)" > "$dict_file"
                      sed -n "/^###### Recommended dictionary/,/^###### End of recommended dictionary/p" "$output_file" \
                        | grep -v "^######" >> "$dict_file"
                      echo "Saved dictionary to $dict_file"
                    fi
                    rm -f "$output_file"
                  }

                  # Run high-risk targets in parallel with dictionary capture
                  run_and_capture fuzz_raft_rpc &
                  run_and_capture fuzz_tui_rpc &
                  run_and_capture fuzz_protocol_handler &
                  run_and_capture fuzz_snapshot_json &
                  wait

                  echo ""
                  echo "Fuzzing complete. Dictionaries saved to fuzz/dictionaries/"
                '
              ''}";
            };

            fuzz-quick = {
              type = "app";
              program = "${pkgs.writeShellScript "fuzz-quick" ''
                set -e

                if [ ! -f "fuzz/Cargo.toml" ]; then
                  echo "Error: Must be run from the aspen project root directory"
                  exit 1
                fi

                echo "Starting quick fuzzing smoke test (5 min per target)..."
                echo ""

                mkdir -p fuzz/dictionaries

                nix develop .#fuzz --command bash -c '
                  for target in fuzz_raft_rpc fuzz_tui_rpc fuzz_snapshot_json; do
                    echo "Fuzzing $target..."

                    # Capture output to extract dictionary
                    output_file=$(mktemp)
                    cargo fuzz run "$target" --sanitizer none -- -max_total_time=300 2>&1 | tee "$output_file"

                    # Extract and save recommended dictionary
                    if grep -q "Recommended dictionary" "$output_file"; then
                      dict_file="fuzz/dictionaries/$target-auto.dict"
                      echo "# Auto-generated dictionary from fuzzing run on $(date -I)" > "$dict_file"
                      sed -n "/^###### Recommended dictionary/,/^###### End of recommended dictionary/p" "$output_file" \
                        | grep -v "^######" >> "$dict_file"
                      echo "Saved dictionary to $dict_file ($(wc -l < "$dict_file") entries)"
                    fi
                    rm -f "$output_file"
                    echo ""
                  done
                '
              ''}";
            };

            fuzz-intensive = {
              type = "app";
              program = "${pkgs.writeShellScript "fuzz-intensive" ''
                set -e

                if [ ! -f "fuzz/Cargo.toml" ]; then
                  echo "Error: Must be run from the aspen project root directory"
                  exit 1
                fi

                echo "Starting intensive fuzzing campaign (6 hours per target)..."
                echo ""

                mkdir -p fuzz/dictionaries

                nix develop .#fuzz --command bash -c '
                  targets=(fuzz_raft_rpc fuzz_tui_rpc fuzz_protocol_handler fuzz_snapshot_json
                           fuzz_log_entries fuzz_gossip fuzz_differential fuzz_http_api)

                  for target in "''${targets[@]}"; do
                    echo "Starting $target (6 hours)..."

                    # Capture output to extract dictionary
                    output_file=$(mktemp)
                    cargo fuzz run "$target" -- -max_total_time=21600 -jobs=4 2>&1 | tee "$output_file"

                    # Extract and save recommended dictionary
                    if grep -q "Recommended dictionary" "$output_file"; then
                      dict_file="fuzz/dictionaries/$target-auto.dict"
                      echo "# Auto-generated dictionary from fuzzing run on $(date -I)" > "$dict_file"
                      sed -n "/^###### Recommended dictionary/,/^###### End of recommended dictionary/p" "$output_file" \
                        | grep -v "^######" >> "$dict_file"
                      echo "Saved dictionary to $dict_file ($(wc -l < "$dict_file") entries)"
                    fi
                    rm -f "$output_file"

                    echo ""
                    echo "Minimizing corpus for $target..."
                    cargo fuzz cmin "$target" || true
                    echo ""
                  done
                '
              ''}";
            };

            fuzz-overnight = {
              type = "app";
              program = "${pkgs.writeShellScript "fuzz-overnight" ''
                set -e

                if [ ! -f "fuzz/Cargo.toml" ]; then
                  echo "Error: Must be run from the aspen project root directory"
                  exit 1
                fi

                echo "Starting overnight fuzzing campaign (8 hours, 4 targets in parallel)..."
                echo "Estimated completion: $(date -d '+8 hours' '+%Y-%m-%d %H:%M')"
                echo ""

                mkdir -p fuzz/dictionaries

                nix develop .#fuzz --command bash -c '
                  # Function to run fuzzer and capture dictionary
                  run_and_capture() {
                    local target=$1
                    local output_file=$(mktemp)

                    echo "[$(date +%H:%M)] Starting $target..."
                    cargo fuzz run "$target" --sanitizer none -- -max_total_time=28800 2>&1 | tee "$output_file"

                    # Extract and save recommended dictionary
                    if grep -q "Recommended dictionary" "$output_file"; then
                      local dict_file="fuzz/dictionaries/$target-auto.dict"
                      echo "# Auto-generated dictionary from overnight fuzzing on $(date -I)" > "$dict_file"
                      sed -n "/^###### Recommended dictionary/,/^###### End of recommended dictionary/p" "$output_file" \
                        | grep -v "^######" >> "$dict_file"
                      echo "[$(date +%H:%M)] Saved dictionary to $dict_file"
                    fi
                    rm -f "$output_file"
                    echo "[$(date +%H:%M)] Completed $target"
                  }

                  # Run 4 high-priority targets in parallel (8 hours each)
                  run_and_capture fuzz_raft_rpc &
                  run_and_capture fuzz_tui_rpc &
                  run_and_capture fuzz_snapshot_json &
                  run_and_capture fuzz_http_api &
                  wait

                  echo ""
                  echo "=== Overnight fuzzing complete ==="
                  echo "Dictionaries saved to fuzz/dictionaries/"
                  echo "Corpus entries:"
                  for target in fuzz_raft_rpc fuzz_tui_rpc fuzz_snapshot_json fuzz_http_api; do
                    count=$(find "fuzz/corpus/$target" -type f 2>/dev/null | wc -l)
                    printf "  %-25s %d entries\n" "$target" "$count"
                  done
                '
              ''}";
            };

            fuzz-corpus = {
              type = "app";
              program = "${pkgs.writeShellScript "fuzz-corpus" ''
                set -e

                if [ ! -f "fuzz/Cargo.toml" ]; then
                  echo "Error: Must be run from the aspen project root directory"
                  exit 1
                fi

                echo "Generating fuzz corpus seeds..."
                nix develop .#fuzz --command cargo run --bin generate_fuzz_corpus --features fuzzing
                echo ""
                echo "Corpus generation complete!"
              ''}";
            };

            # Benchmarking command
            # Usage: nix run .#bench [suite] [filter]
            #   nix run .#bench                    - Run all benchmarks
            #   nix run .#bench production        - Run production benchmarks only
            #   nix run .#bench kv_operations     - Run kv_operations benchmarks only
            #   nix run .#bench storage           - Run storage benchmarks only
            #   nix run .#bench workloads         - Run workload benchmarks only
            #   nix run .#bench concurrency       - Run concurrency benchmarks only
            #   nix run .#bench -- kv_write       - Run all benchmarks matching "kv_write"
            bench = {
              type = "app";
              program = "${pkgs.writeShellScript "bench" ''
                set -e
                SUITE="''${1:-}"
                FILTER="''${2:-}"

                case "$SUITE" in
                  production|kv_operations|storage|workloads|concurrency)
                    if [ -n "$FILTER" ]; then
                      echo "Running $SUITE benchmarks matching: $FILTER"
                      nix develop -c cargo bench --bench "$SUITE" -- "$FILTER"
                    else
                      echo "Running $SUITE benchmarks..."
                      nix develop -c cargo bench --bench "$SUITE"
                    fi
                    ;;
                  "")
                    echo "Running all benchmarks..."
                    nix develop -c cargo bench
                    ;;
                  *)
                    # Treat as filter for all benchmarks
                    echo "Running all benchmarks matching: $SUITE"
                    nix develop -c cargo bench -- "$SUITE"
                    ;;
                esac

                echo ""
                echo "HTML reports available at: target/criterion/report/index.html"
              ''}";
            };

            # Production benchmarks (realistic latencies with SQLite + Iroh)
            # Usage: nix run .#bench-production [filter]
            bench-production = {
              type = "app";
              program = "${pkgs.writeShellScript "bench-production" ''
                set -e
                FILTER="''${1:-}"

                echo "Running production benchmarks (SQLite + Iroh networking)..."
                echo "These show realistic distributed system latencies."
                echo ""

                if [ -n "$FILTER" ]; then
                  nix develop -c cargo bench --bench production -- "$FILTER"
                else
                  nix develop -c cargo bench --bench production
                fi

                echo ""
                echo "HTML reports available at: target/criterion/production/report/index.html"
              ''}";
            };

            # Rust formatter with nightly rustfmt for unstable features
            rustfmt = {
              type = "app";
              program = "${pkgs.writeShellScript "rustfmt" ''
                set -e
                MODE="''${1:-}"
                if [ "$MODE" = "check" ]; then
                  echo "Checking Rust formatting..."
                  ${pkgs.rust-bin.nightly.latest.rustfmt}/bin/rustfmt --edition 2024 --check $(find . -name '*.rs' -not -path './target/*' -not -path './openraft/*' -not -path './vendor/*')
                else
                  echo "Formatting Rust files..."
                  ${pkgs.rust-bin.nightly.latest.rustfmt}/bin/rustfmt --edition 2024 $(find . -name '*.rs' -not -path './target/*' -not -path './openraft/*' -not -path './vendor/*')
                fi
              ''}";
            };

            # Code coverage command
            # Usage: nix run .#coverage [subcommand]
            #   nix run .#coverage        - Show summary (default)
            #   nix run .#coverage html   - Generate HTML report and open browser
            #   nix run .#coverage ci     - Generate lcov.info for CI
            #   nix run .#coverage update - Update .coverage-baseline.toml
            coverage = {
              type = "app";
              program = "${pkgs.writeShellScript "coverage" ''
                set -e

                SUBCMD="''${1:-summary}"

                case "$SUBCMD" in
                  html|open)
                    echo "Generating HTML coverage report..."
                    nix develop -c cargo llvm-cov --html --open --lib \
                      --ignore-filename-regex '(tests/|examples/|openraft/)'
                    ;;

                  ci|lcov)
                    echo "Generating lcov.info for CI..."
                    nix develop -c cargo llvm-cov --lib \
                      --lcov \
                      --output-path lcov.info \
                      --ignore-filename-regex '(tests/|examples/|openraft/)'
                    echo "Coverage report written to lcov.info"
                    ;;

                  update|baseline)
                    echo "Generating coverage data..."
                    nix develop -c cargo llvm-cov --lib \
                      --ignore-filename-regex '(tests/|examples/|openraft/)' \
                      2>&1 | tee /tmp/coverage-output.txt

                    echo ""
                    echo "=== Updating .coverage-baseline.toml ==="

                    TOTAL=$(grep "^TOTAL" /tmp/coverage-output.txt | awk '{print $NF}' | tr -d '%')
                    TOTAL_LINES=$(grep "^TOTAL" /tmp/coverage-output.txt | awk '{print $8}')
                    COVERED_LINES=$(grep "^TOTAL" /tmp/coverage-output.txt | awk '{print $10}')

                    echo "Total coverage: $TOTAL%"
                    echo "Lines: $COVERED_LINES / $TOTAL_LINES"

                    if [ -f .coverage-baseline.toml ]; then
                      sed -i "s/^generated = .*/generated = \"$(date +%Y-%m-%d)\"/" .coverage-baseline.toml
                      sed -i "s/^total_coverage = .*/total_coverage = $TOTAL/" .coverage-baseline.toml
                      echo ""
                      echo "Updated .coverage-baseline.toml"
                      echo "Review changes with: git diff .coverage-baseline.toml"
                    fi
                    ;;

                  summary|"")
                    echo "=== Code Coverage Summary ==="
                    nix develop -c cargo llvm-cov --lib \
                      --ignore-filename-regex '(tests/|examples/|openraft/)'
                    echo ""
                    echo "Commands: nix run .#coverage [html|ci|update|summary]"
                    ;;

                  help|--help|-h)
                    echo "Usage: nix run .#coverage [subcommand]"
                    echo ""
                    echo "Subcommands:"
                    echo "  summary  Show coverage summary to stdout (default)"
                    echo "  html     Generate HTML report and open in browser"
                    echo "  ci       Generate lcov.info for CI/Codecov"
                    echo "  update   Update .coverage-baseline.toml with current coverage"
                    echo "  help     Show this help message"
                    ;;

                  *)
                    echo "Unknown subcommand: $SUBCMD"
                    echo "Run 'nix run .#coverage help' for usage"
                    exit 1
                    ;;
                esac
              ''}";
            };

            fuzz-coverage = {
              type = "app";
              program = "${pkgs.writeShellScript "fuzz-coverage" ''
                set -e

                if [ ! -f "fuzz/Cargo.toml" ]; then
                  echo "Error: Must be run from the aspen project root directory"
                  exit 1
                fi

                mkdir -p fuzz/coverage

                # Parse target from args or use default
                target="''${1:-fuzz_raft_rpc}"

                echo "Generating coverage report for $target..."
                echo ""

                nix develop .#fuzz --command bash -c "
                  # Generate coverage data
                  cargo fuzz coverage \"$target\"

                  # Find the profdata file
                  profdata_dir=\"fuzz/coverage/$target/coverage\"
                  if [ -d \"\$profdata_dir\" ]; then
                    # Merge profile data
                    \$LLVM_PROFDATA merge -sparse \"\$profdata_dir\"/*.profraw -o \"fuzz/coverage/$target.profdata\"

                    # Generate HTML report
                    target_bin=\"fuzz/target/x86_64-unknown-linux-gnu/coverage/x86_64-unknown-linux-gnu/release/$target\"
                    if [ -f \"\$target_bin\" ]; then
                      \$LLVM_COV show \"\$target_bin\" \
                        --instr-profile=\"fuzz/coverage/$target.profdata\" \
                        --format=html \
                        --output-dir=\"fuzz/coverage/$target-html\" \
                        --ignore-filename-regex='/.cargo/|/rustc/'

                      # Also generate summary
                      \$LLVM_COV report \"\$target_bin\" \
                        --instr-profile=\"fuzz/coverage/$target.profdata\" \
                        --ignore-filename-regex='/.cargo/|/rustc/' \
                        > \"fuzz/coverage/$target-summary.txt\"

                      echo \"\"
                      echo \"Coverage report generated:\"
                      echo \"  HTML: fuzz/coverage/$target-html/index.html\"
                      echo \"  Summary: fuzz/coverage/$target-summary.txt\"
                      echo \"\"
                      cat \"fuzz/coverage/$target-summary.txt\"
                    else
                      echo \"Warning: Could not find coverage binary at \$target_bin\"
                    fi
                  else
                    echo \"Warning: No coverage data found. Run fuzzing first.\"
                  fi
                "
              ''}";
            };

            # VM-isolated dogfood cluster
            # Usage: ASPEN_NODE_COUNT=3 nix run .#dogfood-vm
            # Launches N Cloud Hypervisor microVMs (1-10) for isolated CI testing
            dogfood-vm = let
              scriptsDir = pkgs.runCommand "aspen-scripts" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +w $out
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "dogfood-vm" ''
                set -e

                NODE_COUNT=''${ASPEN_NODE_COUNT:-3}
                VM_DIR="$PWD/.aspen/vms"

                # Validate node count (1-10)
                if [ "$NODE_COUNT" -lt 1 ] || [ "$NODE_COUNT" -gt 10 ]; then
                  echo "Error: ASPEN_NODE_COUNT must be 1-10 (got: $NODE_COUNT)"
                  exit 1
                fi

                # Check for KVM
                if [ ! -e /dev/kvm ]; then
                  echo "Error: KVM not available. Please ensure KVM is enabled."
                  echo "  - Check if virtualization is enabled in BIOS"
                  echo "  - Verify with: ls -la /dev/kvm"
                  exit 1
                fi

                mkdir -p "$VM_DIR"

                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-node
                    bins.aspen-cli
                    bins.git-remote-aspen
                    pkgs.cloud-hypervisor
                    pkgs.bash
                    pkgs.coreutils
                    pkgs.curl
                    pkgs.netcat
                    pkgs.gnugrep
                    pkgs.gnused
                    pkgs.iproute2
                    pkgs.bridge-utils
                    pkgs.procps # provides pgrep for process detection
                  ]
                }:$PATH"

                export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                export GIT_REMOTE_ASPEN_BIN="${bins.git-remote-aspen}/bin/git-remote-aspen"
                export ASPEN_NODE_COUNT="$NODE_COUNT"
                export ASPEN_VM_DIR="$VM_DIR"
                # Override PROJECT_DIR since script is in Nix store, not the actual project
                export PROJECT_DIR="$PWD"

                exec ${scriptsDir}/dogfood-vm.sh "$@"
              ''}";
            };
          };
        }
        // {
          # Packages exposed by the flake
          packages =
            {
              default = bins.aspen-node;
              aspen-node = bins.aspen-node;
              aspen-tui = bins.aspen-tui;
              aspen-cli = bins.aspen-cli;
              aspen-ci-agent = bins.aspen-ci-agent;
              git-remote-aspen = bins.git-remote-aspen;
              netwatch = netwatch;
              vm-test-setup = vm-test-setup;
              vm-test-run = vm-test-run;
            }
            // nixpkgs.lib.optionalAttrs (system == "x86_64-linux") (
              let
                # Build the CI worker VM configuration
                ciVmConfig = nixpkgs.lib.nixosSystem {
                  system = "x86_64-linux";
                  modules = [
                    microvm.nixosModules.microvm
                    (import ./nix/vms/ci-worker-node.nix {
                      lib = nixpkgs.lib;
                      vmId = "aspen-ci-vm";
                      aspenCiAgentPackage = bins.aspen-ci-agent;
                    })
                  ];
                };
              in {
                # CI VM kernel for Cloud Hypervisor worker
                ci-vm-kernel = ciVmConfig.config.microvm.kernel;
                # CI VM initrd for Cloud Hypervisor worker
                ci-vm-initrd = ciVmConfig.config.system.build.initialRamdisk;
                # CI VM toplevel (NixOS system with init script)
                # The kernel cmdline needs init=${toplevel}/init to boot NixOS properly
                ci-vm-toplevel = ciVmConfig.config.system.build.toplevel;
                # Full CI VM runner (includes cloud-hypervisor command)
                ci-vm-runner = ciVmConfig.config.microvm.runner.cloud-hypervisor;
              }
            );
        }
        // {
          # Fuzzing development shell with nightly Rust
          # Usage: nix develop .#fuzz
          devShells.fuzz = let
            rustNightly = pkgs.rust-bin.nightly.latest.default.override {
              extensions = ["rust-src" "llvm-tools-preview"];
            };
          in
            pkgs.mkShell {
              buildInputs = with pkgs; [
                rustNightly
                cargo-fuzz
                llvmPackages_latest.llvm
                llvmPackages_latest.libclang
                pkg-config
                openssl.dev
              ];

              LIBCLANG_PATH = "${pkgs.llvmPackages_latest.libclang.lib}/lib";
              LLVM_COV = "${pkgs.llvmPackages_latest.llvm}/bin/llvm-cov";
              LLVM_PROFDATA = "${pkgs.llvmPackages_latest.llvm}/bin/llvm-profdata";

              shellHook = ''
                echo "Fuzzing development environment ready!"
                echo ""
                echo "Nix apps (from project root):"
                echo "  nix run .#fuzz           # Parallel fuzzing (1hr/target)"
                echo "  nix run .#fuzz-quick     # Quick smoke test (5min/target)"
                echo "  nix run .#fuzz-overnight # Overnight run (8hr, 4 parallel targets)"
                echo "  nix run .#fuzz-intensive # Full campaign (6hr/target, sequential)"
                echo "  nix run .#fuzz-coverage  # Generate coverage report"
                echo "  nix run .#fuzz-corpus    # Generate seed corpus"
                echo ""
                echo "Manual commands (in this shell):"
                echo "  cargo fuzz list"
                echo "  cargo fuzz run fuzz_raft_rpc --sanitizer none -- -max_total_time=60"
                echo "  cargo fuzz coverage fuzz_raft_rpc"
                echo ""
                echo "Outputs saved to:"
                echo "  fuzz/corpus/          # Coverage-increasing inputs"
                echo "  fuzz/artifacts/       # Crash-triggering inputs"
                echo "  fuzz/dictionaries/    # Auto-generated dictionaries"
                echo "  fuzz/coverage/        # Coverage reports"
              '';
            };

          devShells.default = craneLib.devShell {
            # Extra inputs can be added here; cargo and rustc are provided by default.
            packages = with pkgs;
              [
                netwatch
                litefs # Transparent SQLite replication via FUSE filesystem
                bash
                coreutils
                cargo-watch
                cargo-nextest
                cargo-llvm-cov # Code coverage tool
                git
                jq
                ripgrep
                rust-analyzer
                sqlite
                pkg-config
                openssl.dev
                zlib # Required by hyperlight-host build script
                codex
                lld # Linker for WASM targets
                mold # Fast linker for development builds
                clang # Used as linker driver for mold
                protobuf # Protocol Buffers compiler for snix crates
                # Pre-commit and quality tools
                pre-commit
                shellcheck
                nodePackages.markdownlint-cli
                # Cloud Hypervisor for VM-based testing
                cloud-hypervisor
                virtiofsd # VirtioFS daemon for VM filesystem sharing
                OVMF # UEFI firmware for Cloud Hypervisor
                # Helper tools for VM management
                bridge-utils # For network bridge management
                iproute2 # For ip commands (TAP devices)
                qemu-utils # For qemu-img disk operations
                # LLVM tools for coverage
                rustc.llvmPackages.llvm
              ]
              ++ [
                # Add our custom helper scripts to devShell
                vm-test-setup
                vm-test-run
              ];

            env.RUST_SRC_PATH = "${rustToolChain}/lib/rustlib/src/rust/library";
            env.SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox}/bin/sh";

            # LLVM coverage tool environment variables
            inherit (pkgs.cargo-llvm-cov) LLVM_COV LLVM_PROFDATA;

            # Incremental compilation settings for faster rebuilds
            env.CARGO_INCREMENTAL = "1";
            env.CARGO_BUILD_INCREMENTAL = "true";

            # Configure cargo to use a shared target directory for better caching
            # This prevents duplicate builds when switching between nix develop and direct cargo commands
            env.CARGO_TARGET_DIR = "target";

            # Enable cargo's new resolver for better dependency resolution
            env.CARGO_RESOLVER = "2";

            # Library path for build scripts that need dynamic libraries (e.g., hyperlight-host needs zlib)
            env.LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [pkgs.zlib pkgs.stdenv.cc.cc.lib];

            # Cloud Hypervisor environment variables
            env.CH_KERNEL = "${pkgs.linuxPackages.kernel}/bzImage";
            env.CH_FIRMWARE = "${pkgs.OVMF.fd}/FV/OVMF.fd";

            # Virtiofsd path for Cloud Hypervisor worker
            env.VIRTIOFSD_PATH = "${pkgs.virtiofsd}/bin/virtiofsd";

            shellHook = ''
              # Auto-stage scripts and decision docs so nix flake sees them
              if [ -d .git ]; then
                git add scripts/*.sh .claude/decisions/*.md 2>/dev/null || true
              fi

              # Set up CI worker environment variables on x86_64-linux
              # These enable the Cloud Hypervisor worker for isolated CI builds
              if [ "$(uname -m)" = "x86_64" ] && [ "$(uname -s)" = "Linux" ]; then
                # Only build CI VM components if not already set
                if [ -z "$ASPEN_CI_KERNEL_PATH" ]; then
                  # Check if CI VM kernel is already built (fast path)
                  CI_KERNEL_STORE=$(nix path-info .#ci-vm-kernel 2>/dev/null || true)
                  if [ -n "$CI_KERNEL_STORE" ]; then
                    export ASPEN_CI_KERNEL_PATH="$CI_KERNEL_STORE/bzImage"
                    export ASPEN_CI_INITRD_PATH=$(nix path-info .#ci-vm-initrd 2>/dev/null)/initrd
                    echo "CI worker: kernel/initrd paths set from cache"
                  else
                    echo "CI worker: Run 'nix build .#ci-vm-kernel .#ci-vm-initrd' to enable VM isolation"
                  fi
                fi
              fi

              echo "Aspen development environment"
              echo ""
              echo "Build caching:"
              echo "  incremental compilation enabled"
              echo ""
              echo "Common commands:"
              echo "  cargo build                          Build project"
              echo "  cargo nextest run                    Run all tests"
              echo "  cargo nextest run -P quick           Quick tests (~2-5 min)"
              echo ""
              echo "Nix apps:"
              echo "  nix run .#cluster                    3-node cluster"
              echo "  nix run .#kitty-hooks-test           Hooks CLI integration test"
              echo "  nix run .#kitty-secrets-test         Secrets CLI integration test"
              echo "  nix run .#bench                      Run benchmarks"
              echo "  nix run .#coverage [html|ci|update]  Code coverage"
              echo "  nix run .#fuzz-quick                 Fuzzing smoke test"
              echo ""
              echo "VM testing: aspen-vm-setup / aspen-vm-run <node-id>"
              echo "  nix run .#dogfood-vm                 Dogfood in isolated VMs"
              echo ""
              echo "CI worker setup (x86_64-linux only):"
              if [ -n "$ASPEN_CI_KERNEL_PATH" ]; then
                echo "  Status: ENABLED (kernel/initrd paths set)"
              else
                echo "  Status: DISABLED (run 'nix build .#ci-vm-kernel .#ci-vm-initrd')"
              fi
            '';
          };
        }
    )
    // {
      # System-independent outputs

      # NixOS modules for Aspen services
      nixosModules = {
        # Aspen node service module
        # Usage: services.aspen.node = { enable = true; nodeId = 1; cookie = "..."; package = ...; };
        aspen-node = import ./nix/modules/aspen-node.nix;

        # Default module includes all Aspen NixOS modules
        default = import ./nix/modules/aspen-node.nix;
      };

      # Library functions for building Aspen infrastructure
      lib = let
        # Use x86_64-linux as the default system for VM builds
        defaultSystem = "x86_64-linux";
        pkgs = import nixpkgs {system = defaultSystem;};
      in {
        # Build a dogfood VM for a specific node ID
        # Returns a NixOS system with microvm configuration
        # The runner is at: result.config.microvm.runner.<hypervisor>
        mkDogfoodVm = {
          nodeId,
          cookie ? "dogfood-vm",
          aspenPackage ? null,
        }:
          nixpkgs.lib.nixosSystem {
            system = defaultSystem;
            modules = [
              microvm.nixosModules.microvm
              ./nix/modules/aspen-node.nix
              (import ./nix/vms/dogfood-node.nix {
                inherit (pkgs) lib;
                inherit nodeId cookie aspenPackage;
              })
            ];
          };
      };
    };
}
