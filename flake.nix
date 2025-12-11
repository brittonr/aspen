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

    # MicroVM framework for VM-based integration testing
    # Uses Cloud Hypervisor as the hypervisor backend for TAP networking
    microvm = {
      url = "github:astro/microvm.nix";
      inputs.nixpkgs.follows = "nixpkgs";
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

        # Build without S3 feature (default)
        aspen = craneLib.buildPackage (
          commonArgs
          // {
            inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
            doCheck = false;
          }
        );

        # Build with S3 feature enabled
        aspen-full = craneLib.buildPackage (
          commonArgs
          // {
            inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
            cargoExtraArgs = "--features s3";
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
          bin = {name}:
            craneLib.buildPackage (
              commonArgs
              // {
                inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                cargoExtraArgs = "--bin ${name}";
                doCheck = false;
              }
            );
          # Special build for aspen-s3 with S3 feature enabled
          aspen-s3 = craneLib.buildPackage (
            commonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
              cargoExtraArgs = "--bin aspen-s3 --features s3";
              doCheck = false;
            }
          );
          bins =
            builtins.listToAttrs (
              map ({name, ...} @ package: lib.nameValuePair name (bin package)) [
                {
                  name = "aspen-node";
                }
                {
                  name = "aspen-tui";
                }
              ]
            )
            // {inherit aspen-s3;};
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
                inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                cargoExtraArgs = "--bin aspen-tui";
                doCheck = false;
              }
            );

            # Convenience alias for the most commonly used dev build
            dev = dev-aspen-node;
          };

        # Docker image for cluster testing (using streamLayeredImage for better caching)
        dockerImage = pkgs.dockerTools.streamLayeredImage {
          name = "aspen-cluster";
          tag = "latest";

          contents = [
            aspen
            flawless
            pkgs.bash
            pkgs.coreutils
            pkgs.gettext # for envsubst
            pkgs.cacert
            # Add entrypoint scripts and config template
            (pkgs.runCommand "aspen-extras" {} ''
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
              fmt = craneLib.cargoFmt basicArgs;
              deny = craneLib.cargoDeny commonArgs;

              audit = craneLib.cargoAudit {
                inherit src advisory-db;
              };

              # NixOS cluster integration test (x86_64-linux only)
              # Run with: nix build .#checks.x86_64-linux.cluster-test
            }
            // (
              if system == "x86_64-linux"
              then {
                cluster-test = import ./nix/checks/cluster-test.nix {
                  inherit self nixpkgs system;
                };
              }
              else {}
            )
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
          apps =
            {
              aspen-node = flake-utils.lib.mkApp {
                drv = bins.aspen-node;
                exePath = "/bin/aspen-node";
              };

              aspen-tui = flake-utils.lib.mkApp {
                drv = bins.aspen-tui;
                exePath = "/bin/aspen-tui";
              };

              # 3-node cluster launcher
              # Usage: nix run .#cluster
              # Environment variables:
              #   ASPEN_NODE_COUNT  - Number of nodes (default: 3)
              #   ASPEN_BASE_HTTP   - Base HTTP port (default: 21001)
              #   ASPEN_STORAGE     - Storage backend: inmemory, sqlite, redb (default: sqlite)
              cluster = {
                type = "app";
                program = "${pkgs.writeShellScript "aspen-cluster" ''
                  export PATH="${
                    pkgs.lib.makeBinPath [
                      bins.aspen-node
                      pkgs.bash
                      pkgs.coreutils
                      pkgs.curl
                      pkgs.netcat
                      pkgs.gnugrep
                    ]
                  }:$PATH"
                  export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                  exec ${./scripts/cluster.sh} "$@"
                ''}";
              };

              default = self.apps.${system}.aspen-node;
            }
            // (
              if system == "x86_64-linux"
              then let
                testCluster = import ./nix/test-cluster.nix {
                  inherit
                    self
                    nixpkgs
                    microvm
                    system
                    ;
                };
              in {
                vm-cluster = testCluster.apps.launch-cluster;
                vm-setup-network = testCluster.apps.setup-network;
                vm-teardown-network = testCluster.apps.teardown-network;
                vm-inject-partition = testCluster.apps.inject-partition;
                vm-heal-partition = testCluster.apps.heal-partition;
              }
              else {}
            );

          # VM-based cluster testing apps (x86_64-linux only)
          # These use Cloud Hypervisor microVMs for true network isolation testing
        }
        // (
          let
            # Base packages available on all systems
            basePackages = {
              default = bins.aspen-node;
              aspen-node = bins.aspen-node;
              aspen-tui = bins.aspen-tui;
              aspen-s3 = bins.aspen-s3; # S3 server binary (requires S3 feature)
              aspen-full = aspen-full; # Full build with all features including S3
              netwatch = netwatch;
              vm-test-setup = vm-test-setup;
              vm-test-run = vm-test-run;
            };
          in {
            # Packages exposed by the flake
            packages =
              basePackages
              // (
                if system == "x86_64-linux"
                then let
                  testCluster = import ./nix/test-cluster.nix {
                    inherit
                      self
                      nixpkgs
                      microvm
                      system
                      ;
                  };
                in {
                  vm-cluster = testCluster.packages.vm-cluster;
                }
                else {}
              );
          }
        )
        // {
          devShells.default = craneLib.devShell {
            # Extra inputs can be added here; cargo and rustc are provided by default.
            packages = with pkgs;
              [
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
                # Cloud Hypervisor for VM-based testing
                cloud-hypervisor
                OVMF # UEFI firmware for Cloud Hypervisor
                # Helper tools for VM management
                bridge-utils # For network bridge management
                iproute2 # For ip commands (TAP devices)
                qemu-utils # For qemu-img disk operations
              ]
              ++ [
                # Add our custom helper scripts to devShell
                vm-test-setup
                vm-test-run
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

            # Cloud Hypervisor environment variables
            env.CH_KERNEL = "${pkgs.linuxPackages.kernel}/bzImage";
            env.CH_FIRMWARE = "${pkgs.OVMF.fd}/FV/OVMF.fd";

            shellHook = ''
              echo "Incremental builds enabled for faster iteration"
              echo "   - Use 'nix build .#dev-aspen-node' for incremental Nix builds"
              echo "   - Use 'cargo build' in this shell for local incremental compilation"
              echo "   - Optional: Run 'export RUSTC_WRAPPER=${pkgs.sccache}/bin/sccache' to enable sccache"
              echo ""
              echo "Cloud Hypervisor VM testing available:"
              echo "   - Run 'aspen-vm-setup' to configure network bridges and TAP devices"
              echo "   - Run 'aspen-vm-run <node-id>' to launch a test VM"
              echo "   - Run 'cloud-hypervisor --version' to check Cloud Hypervisor"
              echo "   - Custom build: 'nix build .#cloud-hypervisor-custom' (from vendored source)"
              echo ""
              echo "Tip: Clean up with 'cargo clean' periodically to prevent disk bloat"
            '';
          };
        }
    )
    // {
      # NixOS modules (system-independent)
      nixosModules = {
        # Aspen node service module
        aspen-node = import ./nix/nixos-modules/aspen-node.nix;

        # Default module that imports all Aspen modules
        default = import ./nix/nixos-modules;
      };

      # MicroVM configurations for testing clusters
      # These are Linux-only configurations using Cloud Hypervisor
      nixosConfigurations = let
        # Helper to create an Aspen node microVM configuration
        makeAspenMicrovm = {
          nodeId,
          httpPort,
          ractorPort,
          macAddress,
          system ? "x86_64-linux",
          additionalModules ? [],
        }: let
          pkgs = import nixpkgs {inherit system;};
        in
          nixpkgs.lib.nixosSystem {
            inherit system;
            modules =
              [
                microvm.nixosModules.microvm
                self.nixosModules.aspen-node
                (
                  {
                    lib,
                    config,
                    ...
                  }: {
                    system.stateVersion = lib.trivial.release;

                    # Basic system configuration
                    networking.hostName = "aspen-node-${toString nodeId}";
                    services.getty.autologinUser = "root";

                    # MicroVM configuration using Cloud Hypervisor
                    microvm = {
                      hypervisor = "cloud-hypervisor";

                      # VM resources
                      vcpu = 2;
                      mem = 512; # MB

                      # Network interface with TAP device for true isolation
                      interfaces = [
                        {
                          type = "tap";
                          id = "aspen-${toString nodeId}";
                          mac = macAddress;
                        }
                      ];

                      # Shared /nix/store from host (read-only)
                      shares = [
                        {
                          tag = "ro-store";
                          source = "/nix/store";
                          mountPoint = "/nix/.ro-store";
                          proto = "virtiofs";
                        }
                      ];

                      # Writable overlay for the store
                      writableStoreOverlay = "/nix/.rw-store";
                      volumes = [
                        {
                          image = "nix-store-overlay.img";
                          mountPoint = config.microvm.writableStoreOverlay;
                          size = 1024; # 1GB
                        }
                        {
                          image = "data.img";
                          mountPoint = "/var/lib/aspen";
                          size = 512; # 512MB for Raft logs and state
                        }
                      ];
                    };

                    # Network configuration
                    networking = {
                      useDHCP = false;
                      interfaces.eth0 = {
                        useDHCP = true;
                      };
                      firewall = {
                        enable = true;
                        allowedTCPPorts = [
                          httpPort
                          ractorPort
                          22
                        ];
                        allowedUDPPortRanges = [
                          {
                            from = 4000;
                            to = 4100;
                          } # Iroh QUIC ports
                        ];
                      };
                    };

                    # Enable SSH for debugging
                    services.openssh = {
                      enable = true;
                      settings.PermitRootLogin = "yes";
                    };

                    # Aspen node service
                    services.aspen-node = {
                      enable = true;
                      package = self.packages.${system}.aspen-node;
                      inherit nodeId;
                      httpAddr = "0.0.0.0:${toString httpPort}";
                      ractorPort = ractorPort;
                      dataDir = "/var/lib/aspen/node-${toString nodeId}";
                      storageBackend = "sqlite";
                      cookie = "aspen-test-cluster";

                      # Enable mDNS for local discovery in the test network
                      iroh.disableMdns = false;
                      iroh.disableGossip = false;

                      environment = {
                        RUST_LOG = "info,aspen=debug";
                        RUST_BACKTRACE = "1";
                      };
                    };
                  }
                )
              ]
              ++ additionalModules;
          };
      in {
        # 3-node test cluster configurations
        "x86_64-linux-aspen-node-1" = makeAspenMicrovm {
          nodeId = 1;
          httpPort = 8301;
          ractorPort = 26001;
          macAddress = "02:00:00:01:01:01";
        };

        "x86_64-linux-aspen-node-2" = makeAspenMicrovm {
          nodeId = 2;
          httpPort = 8302;
          ractorPort = 26002;
          macAddress = "02:00:00:01:01:02";
        };

        "x86_64-linux-aspen-node-3" = makeAspenMicrovm {
          nodeId = 3;
          httpPort = 8303;
          ractorPort = 26003;
          macAddress = "02:00:00:01:01:03";
        };
      };

      # Overlay for microvm packages
      overlays.microvm = final: prev: {
        aspen-microvms = {
          node1 = self.nixosConfigurations."x86_64-linux-aspen-node-1".config.microvm.declaredRunner;
          node2 = self.nixosConfigurations."x86_64-linux-aspen-node-2".config.microvm.declaredRunner;
          node3 = self.nixosConfigurations."x86_64-linux-aspen-node-3".config.microvm.declaredRunner;
        };
      };
    };
}
