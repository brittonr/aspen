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

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    # SNIX - Nix store implementation in Rust
    # Used for content-addressed storage and Nix binary cache integration
    # Tarball URL avoids git clone during flake evaluation — nix verifies
    # the narHash from flake.lock against the store path, so VMs with a
    # shared /nix/store (virtiofs) never hit the network for this input.
    snix-src = {
      url = "https://git.snix.dev/api/v1/repos/snix/snix/archive/e20f82dd6fdebe953fb71bb2fde2f32841015c47.tar.gz";
      flake = false;
    };

    # MicroVM.nix - Declarative NixOS microVMs for isolated dogfood testing
    # Uses Cloud Hypervisor for fast boot times (~125ms) with full kernel isolation
    microvm = {
      url = "github:astro/microvm.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Verus - Formal verification for Rust
    # Built from source to avoid rustup dependency
    verus-src = {
      url = "github:verus-lang/verus/release/0.2026.01.30.44ebdee";
      flake = false;
    };

    # unit2nix - Per-crate Nix builds from Cargo's unit graph
    unit2nix = {
      url = "github:brittonr/unit2nix";
    };
  };

  nixConfig = {
    keepOutputs = true;
    max-jobs = "auto";
    builders = "";
    # ── Binary Caches ─────────────────────────────────────────────────
    # Cargo dependency artifacts (cargoArtifacts, nodeCargoArtifacts,
    # cliCargoArtifacts) are the most expensive derivations (~2000 crates).
    # Push them to a binary cache to save 10-30 min per CI run:
    #
    #   Option A — Cachix (hosted):
    #     $ cachix create aspen
    #     $ nix build .#cargoArtifacts .#nodeCargoArtifacts .#cliCargoArtifacts
    #     $ cachix push aspen ./result
    #
    #   Option B — Attic (self-hosted):
    #     $ attic cache create aspen
    #     $ nix build .#cargoArtifacts .#nodeCargoArtifacts .#cliCargoArtifacts
    #     $ attic push aspen ./result
    #     Then uncomment the substituter/key lines below.
    #
    extra-substituters = [
      "https://cache.nixos.org"
      # microvm.nix provides pre-built Cloud Hypervisor kernels and VM components
      "https://microvm.cachix.org"
      # Uncomment after setting up your project cache:
      # "https://aspen.cachix.org"           # Cachix
      # "https://cache.yourserver.com/aspen"  # Attic / Harmonia
    ];
    extra-trusted-public-keys = [
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      "microvm.cachix.org-1:oXnBc6hRE3eX5rSYdRyMYXnfzcCxC7yKPTbZXALsqys="
      # Uncomment after setting up your project cache:
      # "aspen.cachix.org-1:XXXX..."
    ];
    # Network reliability
    connect-timeout = 30;
    download-attempts = 3;
    fallback = true;
    # Automatic garbage collection
    min-free = 5368709120; # 5GB - trigger GC when less than this free
    max-free = 10737418240; # 10GB - stop GC when this much free
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    crane,
    flake-utils,
    flake-parts,
    advisory-db,
    rust-overlay,
    snix-src,
    microvm,
    verus-src,
    unit2nix,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = flake-utils.lib.defaultSystems;

      imports = [
        ./nix/flake-modules/dev.nix
        ./nix/flake-modules/rust.nix
        ./nix/flake-modules/checks.nix
        ./nix/flake-modules/tests.nix
        ./nix/flake-modules/apps.nix
        ./nix/flake-modules/vms.nix
        ./nix/flake-modules/verus.nix
        ./nix/flake-modules/dogfood.nix
      ];

      # System-independent outputs
      flake = {
        # NixOS modules for Aspen services
        nixosModules = {
          aspen-node = import ./nix/modules/aspen-node.nix;
          default = import ./nix/modules/aspen-node.nix;
        };

        # Library functions for building Aspen infrastructure
        lib = let
          defaultSystem = "x86_64-linux";
          pkgs = import nixpkgs {system = defaultSystem;};
        in {
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

      perSystem = {system, ...}: let
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

        # Use lib.fileset for precise source inclusion — only Rust/Cargo files
        # and vendored sources. Changes to docs, scripts, .agent/, etc. won't
        # trigger rebuilds.
        rawSrc = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./crate-hashes.json
            ./.cargo
            ./.config
            ./build.rs
            ./deny.toml
            ./rustfmt.toml
            ./rust-toolchain.toml
            ./.config
            ./src
            # ./crates  # Included via fullRawSrc for full builds; stubs used for lightweight builds
            # ./openraft  # Included via fullRawSrc for full builds
            ./vendor
            ./tests
            ./benches
            ./examples
            # ./scripts/tutorial-verify  # Extracted to ~/git/aspen-tutorial-verify
          ];
        };

        # ── Full Source Assembly ──────────────────────────────────────
        # For VM integration tests we need the workspace + aspen-wasm-plugin
        # (the only external sibling dep). All other crates live in the workspace.
        # Requires --impure since aspen-wasm-plugin lives outside the flake tree.
        siblingFilter = path: _type: let
          baseName = builtins.baseNameOf path;
        in
          !(baseName == ".git" || baseName == "target" || baseName == ".direnv" || baseName == "result");

        hasImpure = builtins.getEnv "PWD" != "";

        reposDir = let
          env = builtins.getEnv "ASPEN_REPOS_DIR";
          cwd = builtins.getEnv "PWD";
        in
          if env != ""
          then env
          else builtins.dirOf cwd;

        # Full builds need 3 external repos (all other crates are workspace members):
        # - aspen-wasm-plugin: hyperlight WASM plugin runtime (plugins-rpc feature)
        # - aspen-plugins: WASM plugin crates (for VM tests with plugins)
        # - aspen-wasm-guest-sdk: guest SDK for WASM plugins
        hasExternalRepos =
          hasImpure
          && builtins.pathExists (/. + "${reposDir}/aspen-wasm-plugin")
          && builtins.pathExists (/. + "${reposDir}/aspen-plugins")
          && builtins.pathExists (/. + "${reposDir}/aspen-wasm-guest-sdk");

        wasmPluginRepo = builtins.path {
          path = /. + "${reposDir}/aspen-wasm-plugin";
          filter = siblingFilter;
          name = "aspen-sibling-aspen-wasm-plugin";
        };

        snixGitSource = ''source = "git+https://git.snix.dev/snix/snix.git?rev=e20f82dd6fdebe953fb71bb2fde2f32841015c47#e20f82dd6fdebe953fb71bb2fde2f32841015c47"'';
        src = pkgs.runCommand "aspen-src-patched" {} ''
          cp -r ${rawSrc} $out
          chmod -R u+w $out

          # Remove [patch.*] sections from cargo config for Nix builds
          if [ -f $out/.cargo/config.toml ]; then
            ${pkgs.gnused}/bin/sed -i '/^\[patch\./,$d' $out/.cargo/config.toml
          fi

          # Add git source lines to snix packages in Cargo.lock (idempotent)
          for pkg in nix-compat nix-compat-derive snix-castore snix-cli snix-store snix-tracing; do
            ${pkgs.gawk}/bin/awk -v pkg="$pkg" -v src='${snixGitSource}' '
              /^name = "/ && $0 ~ "\"" pkg "\"" { found=1 }
              found && /^version = "0.1.0"$/ {
                print
                if ((getline nextline) > 0) {
                  if (nextline !~ /^source = /) {
                    print src
                  }
                  print nextline
                } else {
                  print src
                }
                found=0
                next
              }
              { print }
            ' $out/Cargo.lock > $out/Cargo.lock.tmp && mv $out/Cargo.lock.tmp $out/Cargo.lock
          done

          # ── Stub crates for extracted sibling repos ─────────────────
          # These repos live at ../aspen-*/ locally but don't exist in the
          # Nix sandbox. Create minimal stub crates inside the source tree
          # at .nix-stubs/ and rewrite all path references to point there.
          stub_crate() {
            local dir="$1" name="$2"
            shift 2
            mkdir -p "$dir/src"
            {
              echo '[package]'
              echo "name = \"$name\""
              echo 'version = "0.1.0"'
              echo 'edition = "2024"'
              echo 'license = "AGPL-3.0-or-later"'
              echo '[features]'
              for feat in "$@"; do
                echo "$feat = []"
              done
            } > "$dir/Cargo.toml"
            echo "// stub for Nix builds" > "$dir/src/lib.rs"
          }

          STUBS="$out/.nix-stubs"

          # Core crates (extracted round 5)
          stub_crate "$STUBS/aspen-core" "aspen-core" layer sql global-discovery
          stub_crate "$STUBS/aspen-client" "aspen-client" forge blob-store
          stub_crate "$STUBS/aspen-cluster" "aspen-cluster" blob docs federation jobs hooks nickel secrets global-discovery bolero
          stub_crate "$STUBS/aspen-auth" "aspen-auth"
          stub_crate "$STUBS/aspen-blob" "aspen-blob"

          # Round 1 extractions (ci, nix, tui, automerge)
          stub_crate "$STUBS/aspen-tui" "aspen-tui"
          stub_crate "$STUBS/aspen-automerge" "aspen-automerge"
          stub_crate "$STUBS/aspen-nickel" "aspen-nickel"
          stub_crate "$STUBS/aspen-ci" "aspen-ci" nickel shell-executor plugins-vm
          stub_crate "$STUBS/aspen-ci-core" "aspen-ci-core"
          stub_crate "$STUBS/aspen-ci-executor-shell" "aspen-ci-executor-shell"
          stub_crate "$STUBS/aspen-ci-executor-nix" "aspen-ci-executor-nix"
          stub_crate "$STUBS/aspen-ci-executor-vm" "aspen-ci-executor-vm"
          stub_crate "$STUBS/aspen-cache" "aspen-cache"
          stub_crate "$STUBS/aspen-snix" "aspen-snix"
          stub_crate "$STUBS/aspen-nix-cache-gateway" "aspen-nix-cache-gateway"
          stub_crate "$STUBS/aspen-nix-handler" "aspen-nix-handler" cache snix

          # Rounds 2-4 extractions
          stub_crate "$STUBS/aspen-layer" "aspen-layer"
          stub_crate "$STUBS/aspen-sql" "aspen-sql"
          stub_crate "$STUBS/aspen-dht-discovery" "aspen-dht-discovery" global-discovery
          stub_crate "$STUBS/aspen-coordination" "aspen-coordination" verus bolero
          stub_crate "$STUBS/aspen-coordination-protocol" "aspen-coordination-protocol"
          stub_crate "$STUBS/aspen-forge" "aspen-forge" git-bridge
          stub_crate "$STUBS/aspen-forge-protocol" "aspen-forge-protocol"
          stub_crate "$STUBS/aspen-secrets" "aspen-secrets"
          stub_crate "$STUBS/aspen-docs" "aspen-docs"
          stub_crate "$STUBS/aspen-hooks" "aspen-hooks"
          stub_crate "$STUBS/aspen-federation" "aspen-federation" global-discovery
          stub_crate "$STUBS/aspen-jobs" "aspen-jobs" blob plugins-vm plugins-wasm
          stub_crate "$STUBS/aspen-jobs-protocol" "aspen-jobs-protocol"
          stub_crate "$STUBS/aspen-jobs-worker-blob" "aspen-jobs-worker-blob"
          stub_crate "$STUBS/aspen-jobs-worker-maintenance" "aspen-jobs-worker-maintenance"
          stub_crate "$STUBS/aspen-jobs-worker-replication" "aspen-jobs-worker-replication"
          stub_crate "$STUBS/aspen-jobs-worker-shell" "aspen-jobs-worker-shell"
          stub_crate "$STUBS/aspen-jobs-worker-sql" "aspen-jobs-worker-sql"

          # Rewrite workspace-level path deps from sibling repos to .nix-stubs/
          ${pkgs.gnused}/bin/sed -i \
            -e 's|path = "\.\./aspen-core/crates/aspen-core"|path = ".nix-stubs/aspen-core"|g' \
            -e 's|path = "\.\./aspen-client/crates/aspen-client"|path = ".nix-stubs/aspen-client"|g' \
            -e 's|path = "\.\./aspen-cluster/crates/aspen-cluster"|path = ".nix-stubs/aspen-cluster"|g' \
            -e 's|path = "\.\./aspen-auth/crates/aspen-auth"|path = ".nix-stubs/aspen-auth"|g' \
            -e 's|path = "\.\./aspen-blob/crates/aspen-blob"|path = ".nix-stubs/aspen-blob"|g' \
            -e 's|path = "\.\./aspen-tui/crates/aspen-tui"|path = ".nix-stubs/aspen-tui"|g' \
            -e 's|path = "\.\./aspen-automerge/crates/aspen-automerge"|path = ".nix-stubs/aspen-automerge"|g' \
            -e 's|path = "\.\./aspen-ci/crates/aspen-nickel"|path = ".nix-stubs/aspen-nickel"|g' \
            -e 's|path = "\.\./aspen-ci/crates/aspen-ci"|path = ".nix-stubs/aspen-ci"|g' \
            -e 's|path = "\.\./aspen-ci/crates/aspen-ci-core"|path = ".nix-stubs/aspen-ci-core"|g' \
            -e 's|path = "\.\./aspen-ci/crates/aspen-ci-executor-shell"|path = ".nix-stubs/aspen-ci-executor-shell"|g' \
            -e 's|path = "\.\./aspen-ci/crates/aspen-ci-executor-nix"|path = ".nix-stubs/aspen-ci-executor-nix"|g' \
            -e 's|path = "\.\./aspen-ci/crates/aspen-ci-executor-vm"|path = ".nix-stubs/aspen-ci-executor-vm"|g' \
            -e 's|path = "\.\./aspen-nix/crates/aspen-cache"|path = ".nix-stubs/aspen-cache"|g' \
            -e 's|path = "\.\./aspen-nix/crates/aspen-snix"|path = ".nix-stubs/aspen-snix"|g' \
            -e 's|path = "\.\./aspen-nix/crates/aspen-nix-cache-gateway"|path = ".nix-stubs/aspen-nix-cache-gateway"|g' \
            -e 's|path = "\.\./aspen-nix/crates/aspen-nix-handler"|path = ".nix-stubs/aspen-nix-handler"|g' \
            -e 's|path = "\.\./aspen-layer/crates/aspen-layer"|path = ".nix-stubs/aspen-layer"|g' \
            -e 's|path = "\.\./aspen-sql/crates/aspen-sql"|path = ".nix-stubs/aspen-sql"|g' \
            -e 's|path = "\.\./aspen-dht-discovery/crates/aspen-dht-discovery"|path = ".nix-stubs/aspen-dht-discovery"|g' \
            -e 's|path = "\.\./aspen-coordination/crates/aspen-coordination"|path = ".nix-stubs/aspen-coordination"|g' \
            -e 's|path = "\.\./aspen-coordination/crates/aspen-coordination-protocol"|path = ".nix-stubs/aspen-coordination-protocol"|g' \
            -e 's|path = "\.\./aspen-forge/crates/aspen-forge"|path = ".nix-stubs/aspen-forge"|g' \
            -e 's|path = "\.\./aspen-forge/crates/aspen-forge-protocol"|path = ".nix-stubs/aspen-forge-protocol"|g' \
            -e 's|path = "\.\./aspen-secrets/crates/aspen-secrets"|path = ".nix-stubs/aspen-secrets"|g' \
            -e 's|path = "\.\./aspen-docs/crates/aspen-docs"|path = ".nix-stubs/aspen-docs"|g' \
            -e 's|path = "\.\./aspen-hooks/crates/aspen-hooks"|path = ".nix-stubs/aspen-hooks"|g' \
            -e 's|path = "\.\./aspen-federation/crates/aspen-federation"|path = ".nix-stubs/aspen-federation"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs"|path = ".nix-stubs/aspen-jobs"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs-protocol"|path = ".nix-stubs/aspen-jobs-protocol"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs-worker-blob"|path = ".nix-stubs/aspen-jobs-worker-blob"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs-worker-maintenance"|path = ".nix-stubs/aspen-jobs-worker-maintenance"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs-worker-replication"|path = ".nix-stubs/aspen-jobs-worker-replication"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs-worker-shell"|path = ".nix-stubs/aspen-jobs-worker-shell"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs-worker-sql"|path = ".nix-stubs/aspen-jobs-worker-sql"|g' \
            $out/Cargo.toml

          # Remove [patch."https://github.com/..."] from Cargo.toml
          # Keep [patch.crates-io] for cargo-hyperlight.
          ${pkgs.gnused}/bin/sed -i '/^\[patch\."https/,$d' $out/Cargo.toml

          # Strip 'layer' feature from aspen-core deps (aspen-layer is a stub in Nix).
          ${pkgs.gnused}/bin/sed -i 's|, features = \["layer"\]||g' \
            $out/Cargo.toml
            # $out/crates/aspen-cli/Cargo.toml  # Extracted to ~/git/aspen-cli
            # $out/crates/aspen-raft/Cargo.toml  # Extracted to ~/git/aspen-raft

          # Also rewrite crate-level path deps that point to sibling repos.
          # From crates/X/, "../../../repo/crates/Y" → "../../.nix-stubs/Y"
          # (Skip if crates/ doesn't exist — all library crates extracted)
          if [ -d "$out/crates" ]; then
          find $out/crates -name Cargo.toml -exec ${pkgs.gnused}/bin/sed -i \
            -e 's|path = "\.\./\.\./\.\./aspen-ci/crates/aspen-ci"|path = "../../.nix-stubs/aspen-ci"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-ci/crates/aspen-nickel"|path = "../../.nix-stubs/aspen-nickel"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-nix/crates/aspen-cache"|path = "../../.nix-stubs/aspen-cache"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-nix/crates/aspen-nix-cache-gateway"|path = "../../.nix-stubs/aspen-nix-cache-gateway"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-nix/crates/aspen-nix-handler"|path = "../../.nix-stubs/aspen-nix-handler"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-layer/crates/aspen-layer"|path = "../../.nix-stubs/aspen-layer"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-sql/crates/aspen-sql"|path = "../../.nix-stubs/aspen-sql"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-dht-discovery/crates/aspen-dht-discovery"|path = "../../.nix-stubs/aspen-dht-discovery"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-coordination/crates/aspen-coordination"|path = "../../.nix-stubs/aspen-coordination"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-coordination/crates/aspen-coordination-protocol"|path = "../../.nix-stubs/aspen-coordination-protocol"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-forge/crates/aspen-forge"|path = "../../.nix-stubs/aspen-forge"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-forge/crates/aspen-forge-protocol"|path = "../../.nix-stubs/aspen-forge-protocol"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-secrets/crates/aspen-secrets"|path = "../../.nix-stubs/aspen-secrets"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-docs/crates/aspen-docs"|path = "../../.nix-stubs/aspen-docs"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-hooks/crates/aspen-hooks"|path = "../../.nix-stubs/aspen-hooks"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-federation/crates/aspen-federation"|path = "../../.nix-stubs/aspen-federation"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs"|path = "../../.nix-stubs/aspen-jobs"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs-protocol"|path = "../../.nix-stubs/aspen-jobs-protocol"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs-worker-blob"|path = "../../.nix-stubs/aspen-jobs-worker-blob"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs-worker-maintenance"|path = "../../.nix-stubs/aspen-jobs-worker-maintenance"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs-worker-replication"|path = "../../.nix-stubs/aspen-jobs-worker-replication"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs-worker-shell"|path = "../../.nix-stubs/aspen-jobs-worker-shell"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs-worker-sql"|path = "../../.nix-stubs/aspen-jobs-worker-sql"|g' \
            {} \;
          fi

        '';

        basicArgs = {
          inherit src;
          inherit pname;
          strictDeps = true;

          # Use --offline because stub crates for extracted sibling repos
          # (.nix-stubs/) cause Cargo.lock drift. Cargo can update the
          # lockfile offline since all deps are vendored.
          cargoExtraArgs = "--offline";

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
            iproute2 # tc for patchbay link conditions
            nftables # nft for patchbay NAT/firewall rules
          ];

          # Ensure libraries are available for build scripts
          LD_LIBRARY_PATH = lib.makeLibraryPath [pkgs.zlib pkgs.stdenv.cc.cc.lib];

          # Set environment variable required by snix-build at compile time
          SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox-sandbox-shell}/bin/busybox";

          # Set PROTO_ROOT for snix-castore build.rs to find proto files
          # Points to the snix source fetched as a flake input
          PROTO_ROOT = "${snix-src}";
        };

        # Vendor cargo dependencies with snix override
        # This substitutes git.snix.dev fetches with our snix-src flake input
        # Subwayrat source (fetched separately to work around stale crane checkout cache)
        subwayratSrc = builtins.fetchGit {
          url = "https://github.com/brittonr/subwayrat";
          rev = "b240e520af26125dda90e351b9a012345e4c9cb9";
          allRefs = true;
        };

        # Must use vendorCargoDeps directly to pass overrideVendorGitCheckout
        cargoVendorDir = craneLib.vendorCargoDeps {
          src = rawSrc;
          overrideVendorGitCheckout = ps: drv: let
            isSnixRepo =
              builtins.any (
                p:
                  builtins.isString (p.source or null)
                  && lib.hasPrefix "git+https://git.snix.dev/snix/snix.git" (p.source or "")
              )
              ps;
            isSubwayrat =
              builtins.any (
                p:
                  builtins.isString (p.source or null)
                  && lib.hasPrefix "git+https://github.com/brittonr/subwayrat" (p.source or "")
              )
              ps;
          in
            if isSnixRepo
            then ensureGitCheckoutLock (drv.overrideAttrs (_old: {src = snix-src;}))
            else if isSubwayrat
            then ensureGitCheckoutLock (drv.overrideAttrs (_old: {src = subwayratSrc;}))
            else ensureGitCheckoutLock drv;
        };

        # ── Cargo Dependency Artifacts ───────────────────────────────
        # Split by feature set so each build variant hits a warm cache.
        # Push all three to your binary cache for fastest CI.

        # Base cargo artifacts — deps only, no feature flags.
        # Used by builds without special features (TUI, CI agent).
        cargoArtifacts = craneLib.buildDepsOnly (
          basicArgs
          // {
            inherit cargoVendorDir;
          }
        );

        # Node-specific cargo artifacts — pre-compiles deps with the full
        # node feature set so aspen-node builds hit warm cache.
        nodeCargoArtifacts = craneLib.buildDepsOnly (
          basicArgs
          // {
            inherit cargoVendorDir;
            pnameSuffix = "-deps-node";
            cargoExtraArgs = "--offline --features ci,docs,hooks,shell-worker,automerge,secrets";
          }
        );

        # CLI-specific cargo artifacts — pre-compiles deps with the union
        # of all CLI feature flags so every CLI variant hits warm cache.
        cliCargoArtifacts = craneLib.buildDepsOnly (
          basicArgs
          // {
            inherit cargoVendorDir;
            pnameSuffix = "-deps-cli";
            cargoExtraArgs = "--offline -p aspen-cli --features forge,secrets,ci,automerge,proxy,sql";
          }
        );

        # Development-specific cargo artifacts that preserve incremental compilation
        # This maintains the Cargo target directory structure for faster rebuilds
        devCargoArtifacts = craneLib.buildDepsOnly (
          basicArgs
          // {
            inherit cargoVendorDir;
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
            inherit cargoArtifacts cargoVendorDir;

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

        # Node-specific arguments — uses nodeCargoArtifacts for faster node builds
        nodeCommonArgs =
          commonArgs
          // {
            cargoArtifacts = nodeCargoArtifacts;
          };

        # CLI-specific arguments — uses cliCommonArgs for faster CLI builds
        cliCommonArgs =
          commonArgs
          // {
            cargoArtifacts = cliCargoArtifacts;
          };

        # ── Full Source Builds (for VM integration tests) ───────────
        # Assembles the real workspace + all sibling repos into one tree
        # that mirrors the actual filesystem layout. No path rewriting needed!
        #
        # Layout (same relative structure as ~/git/):
        #   $out/aspen/         ← main workspace
        #   $out/aspen-core/    ← sibling repo (peer of aspen/)
        #   $out/aspen-auth/    ← sibling repo
        #   ...
        #
        # From $out/aspen/Cargo.toml, ../aspen-core/ resolves naturally.
        # From $out/aspen-X/crates/Y/Cargo.toml, ../../../aspen-Z/ resolves naturally.
        # Full raw source — includes crates/ and openraft/ (excluded from rawSrc
        # to keep lightweight stub builds fast).
        # Rust workspace source — excludes nix/ so test file edits don't
        # invalidate binary derivation caches.
        fullRawSrc = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./.cargo
            ./.config
            ./build.rs
            ./deny.toml
            ./rustfmt.toml
            ./rust-toolchain.toml
            ./flake.nix
            ./flake.lock
            ./src
            ./crates
            ./openraft
            ./vendor
            ./tests
            ./benches
            ./examples
          ];
        };

        # Full source: workspace + aspen-wasm-plugin as a peer.
        # The workspace already contains all crates under crates/.
        # Only aspen-wasm-plugin lives outside (for hyperlight integration).
        fullSrc = pkgs.runCommand "aspen-full-src" {} ''
          mkdir -p $out/aspen

          # Copy main workspace (with all crates) into aspen/ subdirectory
          cp -r ${fullRawSrc}/. $out/aspen/
          chmod -R u+w $out/aspen

          # Remove [patch.*] sections from cargo config for Nix builds
          if [ -f $out/aspen/.cargo/config.toml ]; then
            ${pkgs.gnused}/bin/sed -i '/^\[patch\./,$d' $out/aspen/.cargo/config.toml
          fi

          # Only external sibling dep: aspen-wasm-plugin (for plugins-rpc feature)
          cp -r ${wasmPluginRepo} "$out/aspen-wasm-plugin"

          # Stub aspen-dns (optional dep of aspen-net, dns feature disabled)
          # Path from crates/aspen-net/: ../../../aspen-dns/crates/aspen-dns → $out/aspen-dns/crates/aspen-dns
          mkdir -p "$out/aspen-dns/crates/aspen-dns/src"
          cat > "$out/aspen-dns/crates/aspen-dns/Cargo.toml" << 'EOF'
          [package]
          name = "aspen-dns"
          version = "0.1.0"
          edition = "2024"
          EOF
          echo '// stub' > "$out/aspen-dns/crates/aspen-dns/src/lib.rs"

          chmod -R u+w $out

          # Use the workspace Cargo.lock directly (git deps are stubbed above)

          # Replace ALL git deps with local stubs.
          # Cargo nightly resolves ALL git sources in lockfile even if unused,
          # causing "requires a lock file" vendoring failures.
          stub() {
            local name="$1"; shift
            local dir="$out/aspen/.nix-stubs/$name"
            mkdir -p "$dir/src"
            {
              echo '[package]'
              echo "name = \"$name\""
              echo 'version = "0.1.0"'
              echo 'edition = "2024"'
              echo '[features]'
              for feat in "$@"; do echo "$feat = []"; done
            } > "$dir/Cargo.toml"
            echo '// stub' > "$dir/src/lib.rs"
          }
          stub mad-turmoil
          stub patchbay

          # Rewrite non-snix git deps to path stubs in root Cargo.toml.
          # ALL snix deps (nix-compat, snix-castore, snix-store) stay as git deps
          # because aspen-cache unconditionally depends on nix-compat. Cargo needs
          # all crates from the snix git source resolvable even when the snix
          # feature is off. They're vendored via overrideVendorGitCheckout in
          # fullCargoVendorDir which substitutes the snix-src flake input.
          ${pkgs.gnused}/bin/sed -i \
            -e 's|mad-turmoil = { git = "[^"]*"[^}]*}|mad-turmoil = { path = ".nix-stubs/mad-turmoil", optional = true }|' \
            $out/aspen/Cargo.toml
          ${pkgs.gnused}/bin/sed -i '/dep:mad-turmoil/s/, "dep:mad-turmoil"//g' $out/aspen/Cargo.toml

          # Strip git source lines from Cargo.lock for stubbed deps only.
          # Keep snix.dev, tvlfyi (wu-manber), and subwayrat (aspen-tui) —
          # these are vendored via overrideVendorGitCheckout / normal fetchGit.
          ${pkgs.gnused}/bin/sed -i '/^source = "git+/{
            /snix\.dev/b
            /tvlfyi/b
            /subwayrat/b
            d
          }' $out/aspen/Cargo.lock

          # Rewrite git deps in all subcrates too
          find $out/aspen/crates -name Cargo.toml -exec ${pkgs.gnused}/bin/sed -i \
            -e 's|mad-turmoil = { git = "[^"]*"[^}]*}|mad-turmoil = { path = "../../.nix-stubs/mad-turmoil", optional = true }|' \
            -e 's|patchbay = { git = "[^"]*"[^}]*}|patchbay = { path = "../../.nix-stubs/patchbay" }|' \
            -e 's|bolero = { version = "0.11"|bolero = { version = "0.13"|g' \
            -e 's|bolero-generator = { version = "0.11"|bolero-generator = { version = "0.13"|g' \
            -e 's|bolero = "0.11"|bolero = "0.13"|g' \
            -e 's|bolero-generator = "0.11"|bolero-generator = "0.13"|g' \
            {} \;
        '';

        # ── CI Source Assembly ─────────────────────────────────────
        # Like fullSrc but stubs aspen-wasm-plugin instead of copying the real
        # repo. This means it works in pure evaluation (no --impure, no
        # hasExternalRepos check) while still including all workspace crates.
        # Trades plugins-rpc support for pure reproducibility.
        ciSrc = pkgs.runCommand "aspen-ci-src" {} ''
          mkdir -p $out/aspen

          # Copy main workspace (with all crates) into aspen/ subdirectory
          cp -r ${fullRawSrc}/. $out/aspen/
          chmod -R u+w $out/aspen

          # Remove [patch.*] sections from cargo config for Nix builds
          if [ -f $out/aspen/.cargo/config.toml ]; then
            ${pkgs.gnused}/bin/sed -i '/^\[patch\./,$d' $out/aspen/.cargo/config.toml
          fi

          # Stub aspen-wasm-plugin (optional dep, only for plugins-rpc feature)
          # Path from root Cargo.toml: ../aspen-wasm-plugin/crates/aspen-wasm-plugin
          mkdir -p "$out/aspen-wasm-plugin/crates/aspen-wasm-plugin/src"
          cat > "$out/aspen-wasm-plugin/crates/aspen-wasm-plugin/Cargo.toml" << 'EOF'
          [package]
          name = "aspen-wasm-plugin"
          version = "0.1.0"
          edition = "2024"
          [features]
          default = []
          testing = []
          sql = []
          hooks = []
          EOF
          echo '// stub for CI builds' > "$out/aspen-wasm-plugin/crates/aspen-wasm-plugin/src/lib.rs"


          # Stub aspen-dns (optional dep of aspen-net, dns feature disabled)
          mkdir -p "$out/aspen-dns/crates/aspen-dns/src"
          cat > "$out/aspen-dns/crates/aspen-dns/Cargo.toml" << 'EOF'
          [package]
          name = "aspen-dns"
          version = "0.1.0"
          edition = "2024"
          EOF
          echo '// stub' > "$out/aspen-dns/crates/aspen-dns/src/lib.rs"

          chmod -R u+w $out

          # Stub non-snix git deps as local path crates.
          # snix deps are kept as real git deps — vendored via
          # overrideVendorGitCheckout in ciCargoVendorDir.
          stub() {
            local name="$1"; shift
            local dir="$out/aspen/.nix-stubs/$name"
            mkdir -p "$dir/src"
            {
              echo '[package]'
              echo "name = \"$name\""
              echo 'version = "0.1.0"'
              echo 'edition = "2024"'
              echo '[features]'
              for feat in "$@"; do echo "$feat = []"; done
            } > "$dir/Cargo.toml"
            echo '// stub' > "$dir/src/lib.rs"
          }
          stub mad-turmoil
          stub patchbay

          # Rewrite non-snix git deps to path stubs in root Cargo.toml.
          # snix deps (snix-castore, snix-store, nix-compat) stay as git deps
          # so the cargo vendor dir (with overrideVendorGitCheckout) provides
          # real snix source for aspen-castore, aspen-snix, aspen-snix-bridge.
          ${pkgs.gnused}/bin/sed -i \
            -e 's|mad-turmoil = { git = "[^"]*"[^}]*}|mad-turmoil = { path = ".nix-stubs/mad-turmoil", optional = true }|' \
            $out/aspen/Cargo.toml
          ${pkgs.gnused}/bin/sed -i '/dep:mad-turmoil/s/, "dep:mad-turmoil"//g' $out/aspen/Cargo.toml

          # Strip git source lines from Cargo.lock for stubbed deps only.
          # Keep snix.dev and tvlfyi (wu-manber, a snix dep) source lines —
          # these are vendored via overrideVendorGitCheckout / normal FODs.
          ${pkgs.gnused}/bin/sed -i '/^source = "git+/{
            /snix\.dev/b
            /tvlfyi/b
            d
          }' $out/aspen/Cargo.lock

          # Rewrite git deps in all subcrates too
          find $out/aspen/crates -name Cargo.toml -exec ${pkgs.gnused}/bin/sed -i \
            -e 's|mad-turmoil = { git = "[^"]*"[^}]*}|mad-turmoil = { path = "../../.nix-stubs/mad-turmoil", optional = true }|' \
            -e 's|patchbay = { git = "[^"]*"[^}]*}|patchbay = { path = "../../.nix-stubs/patchbay" }|' \
            -e 's|bolero = { version = "0.11"|bolero = { version = "0.13"|g' \
            -e 's|bolero-generator = { version = "0.11"|bolero-generator = { version = "0.13"|g' \
            -e 's|bolero = "0.11"|bolero = "0.13"|g' \
            -e 's|bolero-generator = "0.11"|bolero-generator = "0.13"|g' \
            {} \;

          # Strip crates not built in CI from workspace members
          # aspen-testing-patchbay: patchbay git dep not vendored
          # aspen-tui: CI doesn't build TUI, git source lines already stripped above
          ${pkgs.gnused}/bin/sed -i '/"crates\/aspen-testing-patchbay"/d' $out/aspen/Cargo.toml
          ${pkgs.gnused}/bin/sed -i '/^aspen-testing-patchbay/d' $out/aspen/Cargo.toml
          ${pkgs.gnused}/bin/sed -i '/"crates\/aspen-tui"/d' $out/aspen/Cargo.toml
          ${pkgs.gnused}/bin/sed -i '/^aspen-tui/d' $out/aspen/Cargo.toml

          # Strip their [[package]] blocks + transitive deps from Cargo.lock
          ${pkgs.python3}/bin/python3 ${pkgs.writeText "strip-ciSrc-lock.py" ''
            import re, sys
            lockfile = sys.argv[1]
            with open(lockfile) as f:
                content = f.read()
            for name in ["aspen-testing-patchbay", "patchbay",
                          "aspen-tui", "rat-cursor", "rat-editor", "rat-event",
                          "rat-ftable", "rat-salsa", "rat-scrolled", "rat-streaming",
                          "rat-table", "rat-text", "rat-widget", "rat-widgets"]:
                pattern = r"\[\[package\]\]\nname = \"" + name + r"\".*?(?=\n\[\[|\Z)"
                content = re.sub(pattern, "", content, flags=re.DOTALL)
            with open(lockfile, "w") as f:
                f.write(content)
          ''} $out/aspen/Cargo.lock
        '';

        # ── CI Build Args ─────────────────────────────────────────────
        # Cargo args for CI checks (nextest, clippy) that work without
        # external repos. Uses ciSrc with stubbed aspen-wasm-plugin.
        ciBasicArgs =
          basicArgs
          // {
            src = ciSrc;
            postUnpack = ''sourceRoot="$sourceRoot/aspen"'';
            cargoToml = ./Cargo.toml;
            # Explicit cargoLock ensures buildDepsOnly includes the lockfile
            # even when src changes invalidate the dummy source derivation.
            cargoLock = ciSrc + "/aspen/Cargo.lock";
            cargoExtraArgs = "";
          };

        ciCargoVendorDir = patchVendorForHyperlight (craneLib.vendorCargoDeps {
          src = ciSrc + "/aspen";
          overrideVendorGitCheckout = ps: drv: let
            isSnixRepo =
              builtins.any (
                p:
                  builtins.isString (p.source or null)
                  && lib.hasPrefix "git+https://git.snix.dev/snix/snix.git" (p.source or "")
              )
              ps;
            isSubwayrat =
              builtins.any (
                p:
                  builtins.isString (p.source or null)
                  && lib.hasPrefix "git+https://github.com/brittonr/subwayrat" (p.source or "")
              )
              ps;
          in
            if isSnixRepo
            then ensureGitCheckoutLock (drv.overrideAttrs (_old: {src = snix-src;}))
            else if isSubwayrat
            then ensureGitCheckoutLock (drv.overrideAttrs (_old: {src = subwayratSrc;}))
            else ensureGitCheckoutLock drv;
        });

        ciCargoArtifacts = craneLib.buildDepsOnly (
          ciBasicArgs
          // {
            cargoVendorDir = ciCargoVendorDir;
            HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
            pnameSuffix = "-ci-deps";
            cargoExtraArgs = "--features ci,docs,hooks,shell-worker,automerge,secrets,proxy";
          }
        );

        ciCommonArgs =
          ciBasicArgs
          // {
            cargoArtifacts = ciCargoArtifacts;
            cargoVendorDir = ciCargoVendorDir;
            HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";

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

        # ── CI-path VM test builds ───────────────────────────────
        # Uses ciSrc instead of fullSrc for VM test binaries.
        # fullSrc suffers from IFD caching issues that prevent snix dep
        # changes from propagating through vendorCargoDeps. The ciSrc path
        # works because it was built correctly from the start.

        # Build aspen-node binary for VM tests via ciSrc path (no plugins)
        ciVmTestBin = {
          name,
          features ? [],
        }:
          craneLib.buildPackage (
            ciCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
              cargoExtraArgs =
                "--bin ${name}"
                + lib.optionalString (features != []) " --features ${lib.concatStringsSep "," features}";
              doCheck = false;
            }
          );

        # Build aspen-cli binary for VM tests via ciSrc path (no plugins)
        ciVmTestCliBin = features:
          craneLib.buildPackage (
            ciCommonArgs
            // {
              pname = "aspen-cli";
              version = "0.1.0";
              cargoExtraArgs =
                "-p aspen-cli --bin aspen-cli"
                + lib.optionalString (features != []) " --features ${lib.concatStringsSep "," features}";
              doCheck = false;
            }
          );

        # Full source with real snix dependencies (not stubs).
        # Required for builds that enable the snix feature.
        # Reverts snix from path stubs back to git deps so the cargo vendor
        # directory (which has the correct snix source) is used.
        fullSrcWithSnix = pkgs.runCommand "aspen-full-src-snix" {} ''
          cp -r ${fullSrc} $out
          chmod -R u+w $out

          # All snix deps (nix-compat, snix-castore, snix-store) are already git deps
          # in fullSrc — no rewriting needed. This just extends fullSrc with the
          # real snix source tree for crates that need snix feature (aspen-snix, etc.).

          # Re-add source lines for snix crates in Cargo.lock
          ${pkgs.gawk}/bin/awk -v src='source = "git+https://git.snix.dev/snix/snix.git?rev=e20f82dd6fdebe953fb71bb2fde2f32841015c47#e20f82dd6fdebe953fb71bb2fde2f32841015c47"' '
            /^name = "/ && ($0 ~ "\"nix-compat\"" || $0 ~ "\"nix-compat-derive\"" || $0 ~ "\"snix-castore\"" || $0 ~ "\"snix-store\"" || $0 ~ "\"snix-cli\"" || $0 ~ "\"snix-tracing\"") { found=1 }
            found && /^version = "0.1.0"$/ {
              print
              if ((getline nextline) > 0) {
                if (nextline !~ /^source = /) {
                  print src
                }
                print nextline
              }
              found=0
              next
            }
            { print }
          ' $out/aspen/Cargo.lock > $out/aspen/Cargo.lock.tmp
          mv $out/aspen/Cargo.lock.tmp $out/aspen/Cargo.lock
        '';

        fullBasicArgs =
          basicArgs
          // {
            src = fullSrc;
            # Enter the aspen/ subdirectory after unpacking so Cargo finds
            # the workspace root. aspen-wasm-plugin at ../aspen-wasm-plugin/ resolves naturally.
            postUnpack = ''sourceRoot="$sourceRoot/aspen"'';
            cargoToml = ./Cargo.toml;
            # Explicit cargoLock ensures buildDepsOnly includes the lockfile
            # in its dummy source — without it cargo can't resolve git dep revisions.
            cargoLock = fullSrc + "/aspen/Cargo.lock";
            cargoExtraArgs = "";
          };

        # Ensure all vendored git checkouts have a Cargo.lock.
        # Cargo requires this for directory-based source replacements of git deps.
        ensureGitCheckoutLock = drv:
          drv.overrideAttrs (old: {
            postInstall =
              (old.postInstall or "")
              + ''
                for d in $out/*/; do
                  if [ -f "$d/Cargo.toml" ] && [ ! -f "$d/Cargo.lock" ]; then
                    echo '# Auto-generated lock for vendored git source' > "$d/Cargo.lock"
                  fi
                done
              '';
          });

        fullCargoVendorDir = craneLib.vendorCargoDeps {
          src = fullSrc + "/aspen";
          overrideVendorGitCheckout = ps: drv: let
            isSnixRepo =
              builtins.any (
                p:
                  builtins.isString (p.source or null)
                  && lib.hasPrefix "git+https://git.snix.dev/snix/snix.git" (p.source or "")
              )
              ps;
            isSubwayrat =
              builtins.any (
                p:
                  builtins.isString (p.source or null)
                  && lib.hasPrefix "git+https://github.com/brittonr/subwayrat" (p.source or "")
              )
              ps;
          in
            if isSnixRepo
            then ensureGitCheckoutLock (drv.overrideAttrs (_old: {src = snix-src;}))
            else if isSubwayrat
            then ensureGitCheckoutLock (drv.overrideAttrs (_old: {src = subwayratSrc;}))
            else ensureGitCheckoutLock drv;
        };

        fullNodeCargoArtifacts = craneLib.buildDepsOnly (
          fullBasicArgs
          // {
            # Use patched vendor dir: hyperlight-wasm build.rs needs pre-built
            # binary even for non-plugins builds (cargo processes all vendor crates)
            cargoVendorDir = fullPluginsCargoVendorDir;
            HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
            pnameSuffix = "-full-deps-node";
            # Build deps for the union of all VM-test feature sets
            cargoExtraArgs = "--features ci,docs,hooks,shell-worker,automerge,secrets,proxy";
          }
        );

        fullCommonArgs =
          fullBasicArgs
          // {
            cargoArtifacts = fullNodeCargoArtifacts;
            cargoVendorDir = fullPluginsCargoVendorDir;
            HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";

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

        fullBin = {
          name,
          features ? [],
        }:
          craneLib.buildPackage (
            fullCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
              cargoExtraArgs =
                "--bin ${name}"
                + lib.optionalString (features != []) " --features ${lib.concatStringsSep "," features}";
              doCheck = false;
            }
          );

        # ── Full CLI builds ────────────────────────────────────────
        # Build aspen-cli from the unified fullSrc workspace. aspen-cli is
        # added as a workspace member so all workspace = true deps resolve
        # correctly against the main workspace.
        fullCliBin = features:
          craneLib.buildPackage (
            fullCommonArgs
            // {
              pname = "aspen-cli";
              version = "0.1.0";
              cargoExtraArgs =
                "-p aspen-cli --bin aspen-cli"
                + lib.optionalString (features != []) " --features ${lib.concatStringsSep "," features}";
              doCheck = false;
            }
          );

        # ── Hyperlight WASM Plugin Support ──────────────────────────────
        # The plugins-rpc feature requires hyperlight-wasm, whose build.rs
        # runs a nested cargo build that fails in Nix's sandbox (no network).
        # We work around this by:
        # 1. Pre-building the wasm_runtime binary in a separate derivation
        # 2. Patching hyperlight-wasm's build.rs to use the pre-built binary
        # 3. Building aspen-node with the patched vendor dir

        # Pre-build the hyperlight wasm_runtime ELF binary
        hyperlight-wasm-runtime = import ./nix/hyperlight-wasm-runtime.nix {
          inherit pkgs lib;
          rustToolChain = rustToolChain;
        };

        # Reusable function: patch a vendor dir's hyperlight-wasm build.rs
        # to use a pre-built wasm_runtime binary instead of nested cargo build.
        patchVendorForHyperlight = baseVendorDir:
          pkgs.runCommand "patched-vendor-for-plugins" {} ''
                      # iroh 0.97 upgrade: force drv hash change to clear stale failure cache
                      cp -rL --no-preserve=mode ${baseVendorDir} $out
                      ${pkgs.gnused}/bin/sed -i "s|${baseVendorDir}|$out|g" $out/config.toml

                      HLW_DIR=$(find $out -maxdepth 3 -type d -name "hyperlight-wasm-0.12.0" | head -1)
                      if [ -z "$HLW_DIR" ]; then
                        echo "ERROR: hyperlight-wasm-0.12.0 not found in vendor dir"
                        exit 1
                      fi

                      cat > "$HLW_DIR/build.rs" << 'BUILDRS'
            use std::path::{Path, PathBuf};
            use std::{env, fs};
            use anyhow::Result;
            use built::write_built_file;

            fn main() -> Result<()> {
                let wasm_runtime_resource = PathBuf::from(
                    env::var("HYPERLIGHT_WASM_RUNTIME")
                        .expect("HYPERLIGHT_WASM_RUNTIME must be set for Nix builds")
                );
                println!("cargo:warning=Using pre-built wasm_runtime from {}", wasm_runtime_resource.display());
                let out_dir = env::var_os("OUT_DIR").unwrap();
                let dest_path = Path::new(&out_dir).join("wasm_runtime_resource.rs");
                let contents = format!(
                    "pub (super) static WASM_RUNTIME: [u8; include_bytes!({name:?}).len()] = *include_bytes!({name:?});",
                    name = wasm_runtime_resource.as_os_str()
                );
                fs::write(dest_path, contents).unwrap();
                let wasm_runtime_bytes = fs::read(&wasm_runtime_resource).unwrap();
                let elf = goblin::elf::Elf::parse(&wasm_runtime_bytes).unwrap();
                let section_name = ".note_hyperlight_metadata";
                let wasmtime_version_number = if let Some(header) = elf.section_headers.iter().find(|hdr| {
                    elf.shdr_strtab.get_at(hdr.sh_name).map_or(false, |name| name == section_name)
                }) {
                    let start = header.sh_offset as usize;
                    let size = header.sh_size as usize;
                    let metadata_bytes = &wasm_runtime_bytes[start..start + size];
                    if let Some(null_pos) = metadata_bytes.iter().position(|&b| b == 0) {
                        std::str::from_utf8(&metadata_bytes[..null_pos]).unwrap()
                    } else {
                        std::str::from_utf8(metadata_bytes).unwrap()
                    }
                } else {
                    panic!(".note_hyperlight_metadata section not found in wasm_runtime binary");
                };
                write_built_file()?;
                let built_path = Path::new(&out_dir).join("built.rs");
                let mut file = std::fs::OpenOptions::new().create(false).append(true).open(built_path).unwrap();
                use std::io::Write;
                let metadata = fs::metadata(&wasm_runtime_resource).unwrap();
                let created = metadata.modified().unwrap();
                let created_datetime: chrono::DateTime<chrono::Local> = created.into();
                writeln!(file, "static WASM_RUNTIME_CREATED: &str = \"{}\";", created_datetime).unwrap();
                writeln!(file, "static WASM_RUNTIME_SIZE: &str = \"{}\";", metadata.len()).unwrap();
                writeln!(file, "static WASM_RUNTIME_WASMTIME_VERSION: &str = \"{}\";", wasmtime_version_number).unwrap();
                let hash = blake3::hash(&wasm_runtime_bytes);
                writeln!(file, "static WASM_RUNTIME_BLAKE3_HASH: &str = \"{}\";", hash).unwrap();
                println!("cargo:rerun-if-changed=build.rs");
                cfg_aliases::cfg_aliases! {
                    gdb: { all(feature = "gdb", debug_assertions) },
                }
                Ok(())
            }
            BUILDRS

                      grep -q "HYPERLIGHT_WASM_RUNTIME" "$HLW_DIR/build.rs"
                      grep -q "$out" "$out/config.toml"
                      echo "Patch applied successfully"
          '';

        # Stub builds: patched vendor for plugins-rpc feature
        pluginsCargoVendorDir = patchVendorForHyperlight cargoVendorDir;

        # Full-source builds: patched vendor for plugins-rpc feature
        fullPluginsCargoVendorDir = patchVendorForHyperlight fullCargoVendorDir;

        fullPluginsCargoArtifacts = craneLib.buildDepsOnly (
          fullBasicArgs
          // {
            pnameSuffix = "-full-deps-node-plugins";
            cargoVendorDir = fullPluginsCargoVendorDir;
            HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
            cargoExtraArgs = "--features ci,docs,hooks,shell-worker,automerge,secrets,plugins-rpc,proxy";
          }
        );

        fullPluginsCommonArgs =
          fullBasicArgs
          // {
            cargoArtifacts = fullPluginsCargoArtifacts;
            cargoVendorDir = fullPluginsCargoVendorDir;
            HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
            nativeBuildInputs =
              basicArgs.nativeBuildInputs
              ++ (with pkgs; [autoPatchelfHook]);
            buildInputs =
              basicArgs.buildInputs
              ++ (lib.optionals pkgs.stdenv.buildPlatform.isDarwin (
                with pkgs; [darwin.apple_sdk.frameworks.Security]
              ));
          };

        # (Old inline pluginsCargoVendorDir replaced by patchVendorForHyperlight above)

        # Cargo artifacts for plugin builds (uses patched vendor dir)
        # Must NOT inherit from cargoArtifacts — the patched build.rs
        # must be compiled fresh, not reused from the original cache.
        pluginsCargoArtifacts = craneLib.buildDepsOnly (
          basicArgs
          // {
            pname = "aspen-plugins";
            cargoVendorDir = pluginsCargoVendorDir;
            HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
            cargoExtraArgs = "--features plugins-rpc";
          }
        );

        # Common args for plugins-enabled builds
        pluginsCommonArgs =
          basicArgs
          // {
            cargoVendorDir = pluginsCargoVendorDir;
            cargoArtifacts = pluginsCargoArtifacts;
            HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
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

        # ── WASM Plugin Builds ──────────────────────────────────────
        # Builds cdylib WASM plugins for VM integration tests.
        # Requires --impure for aspen-plugins + aspen-wasm-guest-sdk repos.
        # Dependency chain: plugins → aspen-wasm-guest-sdk → aspen-client-api
        #   → aspen-plugin-api (both from main workspace crates/)
        pluginsRepo = builtins.path {
          path = /. + "${reposDir}/aspen-plugins";
          filter = siblingFilter;
          name = "aspen-sibling-aspen-plugins";
        };

        guestSdkRepo = builtins.path {
          path = /. + "${reposDir}/aspen-wasm-guest-sdk";
          filter = siblingFilter;
          name = "aspen-sibling-aspen-wasm-guest-sdk";
        };

        wasmPluginsSrc = pkgs.runCommand "wasm-plugins-src" {} ''
          mkdir -p $out

          # Copy the three repos needed for plugin builds
          cp -r ${pluginsRepo} $out/aspen-plugins
          cp -r ${guestSdkRepo} $out/aspen-wasm-guest-sdk
          # Main workspace provides aspen-client-api, aspen-plugin-api, and
          # all transitive deps (protocol crates etc.) via crates/
          # Must use fullRawSrc (includes ./crates and ./openraft), not rawSrc (stubs only)
          cp -r ${fullRawSrc} $out/aspen

          chmod -R u+w $out
          # Use pre-generated Cargo.lock with all plugin crates
          cp ${./nix/plugins-Cargo.lock} $out/aspen-plugins/Cargo.lock
        '';

        wasmPluginsCargoVendorDir = craneLib.vendorCargoDeps {
          src = wasmPluginsSrc + "/aspen-plugins";
        };

        buildWasmPlugin = {
          name, # e.g. "coordination"
          crateName, # e.g. "aspen-coordination-plugin"
        }:
          pkgs.stdenv.mkDerivation {
            pname = "aspen-${name}-plugin-wasm";
            version = "0.1.0";
            src = wasmPluginsSrc;
            nativeBuildInputs = [rustToolChain pkgs.lld];
            CARGO_HOME = "$TMPDIR/cargo-home";
            buildPhase = ''
              mkdir -p $CARGO_HOME
              mkdir -p aspen-plugins/.cargo
              cp ${wasmPluginsCargoVendorDir}/config.toml aspen-plugins/.cargo/config.toml
              cd aspen-plugins
              cargo build \
                --release \
                --target wasm32-unknown-unknown \
                --offline \
                -p ${crateName}
            '';
            installPhase = ''
              mkdir -p $out
              WASM_NAME=$(echo "${crateName}" | tr '-' '_')
              # CWD is aspen-plugins/ from buildPhase cd, so target/ is relative
              cp target/wasm32-unknown-unknown/release/$WASM_NAME.wasm \
                 $out/${name}-plugin.wasm
              cp crates/${crateName}/plugin.json \
                 $out/plugin.json
            '';
          };

        coordinationPluginWasm = buildWasmPlugin {
          name = "coordination";
          crateName = "aspen-coordination-plugin";
        };

        automergePluginWasm = buildWasmPlugin {
          name = "automerge";
          crateName = "aspen-automerge-plugin";
        };

        secretsPluginWasm = buildWasmPlugin {
          name = "secrets";
          crateName = "aspen-secrets-plugin";
        };

        serviceRegistryPluginWasm = buildWasmPlugin {
          name = "service-registry";
          crateName = "aspen-service-registry-plugin";
        };

        kvPluginWasm = buildWasmPlugin {
          name = "kv";
          crateName = "aspen-kv-plugin";
        };

        forgePluginWasm = buildWasmPlugin {
          name = "forge";
          crateName = "aspen-forge-plugin";
        };

        hooksPluginWasm = buildWasmPlugin {
          name = "hooks";
          crateName = "aspen-hooks-plugin";
        };

        sqlPluginWasm = buildWasmPlugin {
          name = "sql";
          crateName = "aspen-sql-plugin";
        };

        # Build the main package
        aspen = craneLib.buildPackage (
          commonArgs
          // {
            inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
            doCheck = false;
          }
        );

        # Minimal busybox initramfs for VirtioFS integration testing.
        # Mounts virtiofs tag "testfs", writes/reads a file, prints PASS/FAIL,
        # then powers off. Used by cloud_hypervisor_virtiofs_test.rs.
        virtiofsTestInitrd = let
          kmod = pkgs.linuxPackages.kernel.modules;
          modDir = "${kmod}/lib/modules/${pkgs.linuxPackages.kernel.modDirVersion}/kernel";
          fuseDir = "${modDir}/fs/fuse";
          virtioDir = "${modDir}/drivers/virtio";
        in
          pkgs.runCommand "virtiofs-test-initrd" {
            nativeBuildInputs = with pkgs; [cpio gzip];
          } ''
                      mkdir -p root/{bin,dev,proc,sys,mnt,lib/modules}
                      cp ${pkgs.pkgsStatic.busybox}/bin/busybox root/bin/busybox
                      for cmd in sh mount umount echo insmod poweroff cat mkdir; do
                        ln -sf busybox root/bin/$cmd
                      done
                      # FUSE modules
                      cp ${fuseDir}/fuse.ko.xz root/lib/modules/
                      cp ${fuseDir}/virtiofs.ko.xz root/lib/modules/
                      # Virtio transport modules (virtiofs depends on virtio_ring, virtio, virtio_pci)
                      cp ${virtioDir}/virtio.ko.xz root/lib/modules/
                      cp ${virtioDir}/virtio_ring.ko.xz root/lib/modules/
                      cp ${virtioDir}/virtio_pci_modern_dev.ko.xz root/lib/modules/
                      cp ${virtioDir}/virtio_pci_legacy_dev.ko.xz root/lib/modules/
                      cp ${virtioDir}/virtio_pci.ko.xz root/lib/modules/
                      cat > root/init << 'INITEOF'
            #!/bin/sh
            mount -t proc none /proc
            mount -t sysfs none /sys
            mount -t devtmpfs none /dev
            # Load virtio transport stack (order matters: deps first)
            insmod /lib/modules/virtio.ko.xz
            insmod /lib/modules/virtio_ring.ko.xz
            insmod /lib/modules/virtio_pci_modern_dev.ko.xz
            insmod /lib/modules/virtio_pci_legacy_dev.ko.xz
            insmod /lib/modules/virtio_pci.ko.xz
            # Load FUSE + virtiofs
            insmod /lib/modules/fuse.ko.xz
            insmod /lib/modules/virtiofs.ko.xz
            # Wait briefly for virtio-fs PCI device to be probed
            sleep 1
            mount -t virtiofs testfs /mnt
            if [ $? -ne 0 ]; then
              echo "VIRTIOFS_TEST_FAIL: mount failed"
              poweroff -f
            fi
            echo "hello from guest" > /mnt/result.txt
            READBACK=$(cat /mnt/result.txt)
            if [ "$READBACK" = "hello from guest" ]; then
              echo "VIRTIOFS_TEST_PASS"
            else
              echo "VIRTIOFS_TEST_FAIL: got '$READBACK'"
            fi
            poweroff -f
            INITEOF
                      chmod +x root/init
                      cd root && find . | cpio --quiet -H newc -o | gzip -9 > $out
          '';

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

        # ==================================================================
        # Verus Formal Verification Tool
        # ==================================================================
        # Build Z3 4.12.5 from source (Verus requires this exact version)
        z3_4_12_5 = pkgs.z3.overrideAttrs (old: rec {
          version = "4.12.5";
          src = pkgs.fetchFromGitHub {
            owner = "Z3Prover";
            repo = "z3";
            rev = "z3-${version}";
            hash = "sha256-Qj9w5s02OSMQ2qA7HG7xNqQGaUacA1d4zbOHynq5k+A=";
          };
        });

        # Rust toolchain for Verus (requires Rust 1.93.0 with rustc-dev and llvm-tools)
        verusRustToolchain = pkgs.rust-bin.stable."1.93.0".default.override {
          extensions = ["rustc-dev" "llvm-tools" "rust-src"];
        };

        # Verus development shell - provides all dependencies for building/running Verus
        # Verus has a complex build system (vargo) with git dependencies that makes
        # direct Nix packaging difficult. This shell provides the environment
        # needed to build Verus from source or run pre-built binaries.
        verusDevShell = pkgs.mkShell {
          name = "verus-dev";
          nativeBuildInputs = with pkgs; [
            verusRustToolchain
            z3_4_12_5
            pkg-config
            openssl
            git
          ];
          shellHook = ''
            export VERUS_Z3_PATH="${z3_4_12_5}/bin/z3"
            echo "Verus development environment"
            echo "  Z3: ${z3_4_12_5}/bin/z3"
            echo "  Rust: 1.93.0 with rustc-dev, llvm-tools, rust-src"
            echo ""
            echo "To build Verus from source:"
            echo "  git clone https://github.com/verus-lang/verus"
            echo "  cd verus"
            echo "  source tools/activate"
            echo "  cd source && vargo build --release"
          '';
        };

        # Fetch pre-built Verus binaries from GitHub releases
        # Verus has a complex build system (vargo) that makes direct Nix packaging difficult,
        # but the binary releases work perfectly with proper library path setup.
        verusRoot = pkgs.stdenv.mkDerivation {
          pname = "verus-root";
          version = "0.2026.01.30";

          src = pkgs.fetchzip {
            url = "https://github.com/verus-lang/verus/releases/download/release%2F0.2026.01.30.44ebdee/verus-0.2026.01.30.44ebdee-x86-linux.zip";
            hash = "sha256-oRStgYjHmQS4zZ4iBpSRrpaS1/dMV65RdFKOeVqejUo=";
            stripRoot = false;
          };

          nativeBuildInputs = [pkgs.autoPatchelfHook];
          buildInputs = [pkgs.stdenv.cc.cc.lib pkgs.zlib];

          # Rust libs come from verusRustToolchain at runtime, so ignore these
          autoPatchelfIgnoreMissingDeps = ["librustc_driver*" "libLLVM*" "libstd-*"];

          installPhase = ''
            mkdir -p $out/bin $out/lib
            # The zip extracts with verus-x86-linux subdirectory - copy its contents to root
            cp -r verus-x86-linux/* $out/
            # Ensure binaries are executable
            chmod +x $out/rust_verify $out/verus $out/cargo-verus $out/z3 2>/dev/null || true
          '';
        };

        # Verus wrapper that sets up the environment correctly
        # The rust_verify binary needs librustc_driver from Rust 1.93.0
        # We use rust_verify directly since the 'verus' binary expects rustup
        verus = pkgs.writeShellScriptBin "verus" ''
          # Set library path for Rust 1.93.0 compiler libs (provides librustc_driver)
          export LD_LIBRARY_PATH="${verusRustToolchain}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

          # Set Z3 path to use bundled Z3 from the release
          export VERUS_Z3_PATH="${verusRoot}/z3"

          # Set VERUS_ROOT so rust_verify can find vstd and builtin libraries
          export VERUS_ROOT="${verusRoot}"

          # Call rust_verify directly (the verus binary expects rustup, which we don't have)
          exec "${verusRoot}/rust_verify" "$@"
        '';

        # ── unit2nix: Per-crate Nix builds ────────────────────────────
        # Uses auto mode (IFD) — build plans generated at eval time from
        # Cargo's unit graph. No checked-in JSON files to maintain.
        # Changing one crate only rebuilds its dependents, not the entire
        # 648-crate workspace.
        u2nRawSrc = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./crate-hashes.json
            ./build.rs
            ./src
            ./crates
            ./openraft
            ./tests
            ./benches
            ./vendor
          ];
        };

        # Strip external path deps (aspen-dns, aspen-wasm-plugin) from
        # the workspace so cargo doesn't try to resolve them in the IFD
        # sandbox. Both are optional and never activated by our feature sets.
        # Also strip their entries from Cargo.lock so --locked passes.
        u2nSrc = pkgs.runCommand "aspen-u2n-src" {} ''
          mkdir -p $out/aspen
          cp -r ${u2nRawSrc}/* $out/aspen/
          chmod -R u+w $out/aspen

          # Remove external optional deps: aspen-wasm-plugin, aspen-dns
          # Strip path deps, feature activations, and Cargo.lock entries.
          # These are never activated by our feature sets (no plugins-rpc, no dns).
          ${pkgs.python3}/bin/python3 ${pkgs.writeText "strip-external-deps.py" ''
            import re, sys
            out = sys.argv[1]

            def strip_lines(path, patterns):
                with open(path) as f:
                    lines = f.readlines()
                with open(path, "w") as f:
                    for line in lines:
                        if not any(p in line for p in patterns):
                            f.write(line)

            # Root Cargo.toml: remove wasm-plugin dep + plugin features
            strip_lines(f"{out}/aspen/Cargo.toml", [
                'aspen-wasm-plugin = { path',
                'plugins-rpc = [',
                'plugins = ["plugins-vm"',
                'plugins-vm = [',
                'plugins-wasm = [',
            ])
            # Strip "plugins" from the full feature list
            root_toml = f"{out}/aspen/Cargo.toml"
            with open(root_toml) as f:
                content = f.read()
            content = content.replace('"plugins", ', "")
            with open(root_toml, "w") as f:
                f.write(content)

            # aspen-rpc-handlers: remove wasm-plugin dep + features referencing it
            strip_lines(f"{out}/aspen/crates/aspen-rpc-handlers/Cargo.toml", [
                'aspen-wasm-plugin = { path',
                'plugins-rpc = [',
            ])
            # Also strip aspen-wasm-plugin?/hooks from the hooks feature line
            rpc_toml = f"{out}/aspen/crates/aspen-rpc-handlers/Cargo.toml"
            with open(rpc_toml) as f:
                content = f.read()
            content = content.replace(', "aspen-wasm-plugin?/hooks"', "")
            with open(rpc_toml, "w") as f:
                f.write(content)

            # aspen-net: remove aspen-dns dep + dns feature
            strip_lines(f"{out}/aspen/crates/aspen-net/Cargo.toml", [
                'aspen-dns = { path',
                'dns = ["dep:aspen-dns"',
            ])
          ''} $out

          # Strip aspen-wasm-plugin, aspen-dns, aspen-testing-patchbay package blocks from Cargo.lock
          ${pkgs.python3}/bin/python3 ${pkgs.writeText "strip-external-deps-lock.py" ''
            import re, sys
            lockfile = sys.argv[1]
            with open(lockfile) as f:
                content = f.read()
            for name in ["aspen-wasm-plugin", "aspen-dns", "aspen-testing-patchbay", "aspen-tui"]:
                pattern = r"\[\[package\]\]\nname = \"" + name + r"\".*?(?=\n\[\[|\Z)"
                content = re.sub(pattern, "", content, flags=re.DOTALL)
            # Also strip git dep blocks not vendored in ciSrc
            for name in ["patchbay", "rat-cursor", "rat-editor", "rat-event", "rat-ftable",
                          "rat-salsa", "rat-scrolled", "rat-streaming", "rat-table", "rat-text",
                          "rat-widget", "rat-widgets"]:
                pattern = r"\[\[package\]\]\nname = \"" + name + r"\".*?(?=\n\[\[|\Z)"
                content = re.sub(pattern, "", content, flags=re.DOTALL)
            with open(lockfile, "w") as f:
                f.write(content)
          ''} $out/aspen/Cargo.lock

          # Strip aspen-testing-patchbay from workspace members (test-only crate with git dep)
          ${pkgs.gnused}/bin/sed -i '/"crates\/aspen-testing-patchbay"/d' $out/aspen/Cargo.toml
          ${pkgs.gnused}/bin/sed -i '/^aspen-testing-patchbay/d' $out/aspen/Cargo.toml

          # Strip aspen-tui from workspace members (CI doesn't build TUI)
          ${pkgs.gnused}/bin/sed -i '/"crates\/aspen-tui"/d' $out/aspen/Cargo.toml
          ${pkgs.gnused}/bin/sed -i '/^aspen-tui/d' $out/aspen/Cargo.toml

          # Rewrite patchbay git dep in any remaining subcrates
          find $out/aspen/crates -name Cargo.toml -exec ${pkgs.gnused}/bin/sed -i \
            -e 's|patchbay = { git = "[^"]*"[^}]*}|patchbay = { path = "../../.nix-stubs/patchbay" }|' \
            {} \;

          # Create patchbay stub (referenced by aspen-dag, aspen-forge dev-deps)
          mkdir -p $out/aspen/.nix-stubs/patchbay/src
          cat > $out/aspen/.nix-stubs/patchbay/Cargo.toml <<'TOML'
          [package]
          name = "patchbay"
          version = "0.1.0"
          edition = "2024"
          TOML
          echo '// stub' > $out/aspen/.nix-stubs/patchbay/src/lib.rs

          # Strip patchbay git source lines from Cargo.lock
          ${pkgs.gnused}/bin/sed -i '/^source = "git+.*patchbay/d' $out/aspen/Cargo.lock
        '';

        # Extended source for workspace-mode test builds.
        # Workspace mode resolves ALL members, including those with git deps
        # (snix, iroh-h3, iroh-proxy-utils, mad-turmoil) that can't be fetched
        # in the IFD sandbox. Stub them as empty path crates.
        u2nTestSrc = pkgs.runCommand "aspen-u2n-test-src" {} ''
          cp -r ${u2nSrc} $out
          chmod -R u+w $out

          # Stub git deps as empty crates under .nix-stubs/
          stub() {
            local name="$1"; shift
            local dir="$out/aspen/.nix-stubs/$name"
            mkdir -p "$dir/src"
            {
              echo '[package]'
              echo "name = \"$name\""
              echo 'version = "0.1.0"'
              echo 'edition = "2024"'
              echo '[features]'
              for feat in "$@"; do echo "$feat = []"; done
            } > "$dir/Cargo.toml"
            echo '// stub' > "$dir/src/lib.rs"
          }
          stub snix-castore
          stub snix-store
          stub nix-compat async serde flakeref
          stub nix-compat-derive
          stub snix-build
          stub snix-eval
          stub snix-glue
          stub snix-serde
          stub nix-daemon
          stub nar-bridge
          stub snix-castore-http
          stub snix-tracing clap otlp
          stub mad-turmoil
          stub patchbay

          # Rewrite ALL git deps to path stubs in root Cargo.toml
          ${pkgs.gnused}/bin/sed -i \
            -e 's|snix-castore = { git = "[^"]*"[^}]*}|snix-castore = { path = ".nix-stubs/snix-castore" }|' \
            -e 's|snix-store = { git = "[^"]*"[^}]*}|snix-store = { path = ".nix-stubs/snix-store" }|' \
            -e 's|nix-compat = { git = "[^"]*"[^}]*}|nix-compat = { path = ".nix-stubs/nix-compat", features = ["async", "serde"] }|' \
            -e 's|snix-build = { git = "[^"]*"[^}]*}|snix-build = { path = ".nix-stubs/snix-build" }|' \
            -e 's|snix-eval = { git = "[^"]*"[^}]*}|snix-eval = { path = ".nix-stubs/snix-eval" }|' \
            -e 's|snix-glue = { git = "[^"]*"[^}]*}|snix-glue = { path = ".nix-stubs/snix-glue" }|' \
            -e 's|snix-serde = { git = "[^"]*"[^}]*}|snix-serde = { path = ".nix-stubs/snix-serde" }|' \
            -e 's|nix-daemon = { git = "[^"]*"[^}]*}|nix-daemon = { path = ".nix-stubs/nix-daemon" }|' \
            -e 's|nar-bridge = { git = "[^"]*"[^}]*}|nar-bridge = { path = ".nix-stubs/nar-bridge" }|' \
            -e 's|snix-castore-http = { git = "[^"]*"[^}]*}|snix-castore-http = { path = ".nix-stubs/snix-castore-http" }|' \
            -e 's|snix-tracing = { git = "[^"]*"[^}]*}|snix-tracing = { path = ".nix-stubs/snix-tracing" }|' \
            -e 's|mad-turmoil = { git = "[^"]*"[^}]*}|mad-turmoil = { path = ".nix-stubs/mad-turmoil", optional = true }|' \
            $out/aspen/Cargo.toml
          ${pkgs.gnused}/bin/sed -i '/dep:mad-turmoil/s/, "dep:mad-turmoil"//g' $out/aspen/Cargo.toml

          # Rewrite git deps in subcrates
          find $out/aspen/crates -name Cargo.toml -exec ${pkgs.gnused}/bin/sed -i \
            -e 's|mad-turmoil = { git = "[^"]*"[^}]*}|mad-turmoil = { path = "../../.nix-stubs/mad-turmoil", optional = true }|' \
            -e 's|patchbay = { git = "[^"]*"[^}]*}|patchbay = { path = "../../.nix-stubs/patchbay" }|' \
            {} \;

          # Strip git source lines from Cargo.lock (stubs are path deps now)
          ${pkgs.gnused}/bin/sed -i '/^source = "git+/d' $out/aspen/Cargo.lock

          # Strip aspen-tui from workspace (u2n sandbox can't fetch git deps)
          ${pkgs.gnused}/bin/sed -i '/"crates\/aspen-tui"/d' $out/aspen/Cargo.toml
          ${pkgs.gnused}/bin/sed -i '/^aspen-tui/d' $out/aspen/Cargo.toml
        '';

        # Shared crate overrides for all unit2nix builds.
        u2nCrateOverrides =
          pkgs.defaultCrateOverrides
          // {
            # Root aspen crate: build.rs needs git + date
            aspen = attrs: {
              nativeBuildInputs = (attrs.nativeBuildInputs or []) ++ [pkgs.git];
              GIT_HASH = self.shortRev or self.dirtyShortRev or "nix";
              BUILD_TIME = "1970-01-01 00:00:00 UTC";
            };
            # aspen-cli: build.rs also needs git + date (same pattern)
            aspen-cli = attrs: {
              nativeBuildInputs = (attrs.nativeBuildInputs or []) ++ [pkgs.git];
              GIT_HASH = self.shortRev or self.dirtyShortRev or "nix";
              BUILD_TIME = "1970-01-01 00:00:00 UTC";
            };
            # Crates whose descriptions contain embedded double quotes, which break
            # buildRustCrate's `export CARGO_PKG_DESCRIPTION="..."` in bash.
            base16ct = _: {description = builtins.replaceStrings [''"''] ["'"] (_.description or "");};
            base64ct = _: {description = builtins.replaceStrings [''"''] ["'"] (_.description or "");};
            cobs = _: {description = builtins.replaceStrings [''"''] ["'"] (_.description or "");};
            leb128 = _: {description = builtins.replaceStrings [''"''] ["'"] (_.description or "");};
            openssl-probe = _: {description = builtins.replaceStrings [''"''] ["'"] (_.description or "");};
            ssh-key = _: {description = builtins.replaceStrings [''"''] ["'"] (_.description or "");};
            syn-mid = _: {description = builtins.replaceStrings [''"''] ["'"] (_.description or "");};
            zerocopy = _: {description = builtins.replaceStrings [''"''] ["'"] (_.description or "");};
            # ring: needs cc + LLVM for assembly.
            ring = attrs: {
              RING_CORE_PREFIX = "";
            };
            # aspen-sops: no special overrides needed (keyservice removed).
          };

        # Common args shared across all auto-mode builds
        u2nAutoCommon = {
          inherit pkgs;
          src = u2nSrc;
          workspaceDir = "aspen";
          defaultCrateOverrides = u2nCrateOverrides;
          rustToolchain = rustToolChain;
          # External optional deps (aspen-wasm-plugin, aspen-dns) are stripped
          # from the manifest but their transitive deps remain in Cargo.lock.
          noLocked = true;
        };

        # Per-crate test builds: workspace = true resolves ALL members with
        # dev-deps, enabling test.check.<member> for every workspace crate.
        # Uses u2nTestSrc (git deps stubbed) so IFD sandbox can resolve all members.
        # Only used for flake checks — binary builds use the narrower plans above.
        u2nTestWorkspace = unit2nix.lib.${system}.buildFromUnitGraphAuto (u2nAutoCommon
          // {
            src = u2nTestSrc;
            workspace = true;
          });

        # Short aliases used throughout the flake
        aspenNode = u2nWorkspace.workspaceMembers."aspen".build;
        aspenCli = u2nCliWorkspace.workspaceMembers."aspen-cli".build;

        # aspen-node (features: ci,docs,hooks,shell-worker,automerge,secrets,git-bridge,deploy)
        u2nWorkspace = unit2nix.lib.${system}.buildFromUnitGraphAuto (u2nAutoCommon
          // {
            bin = "aspen-node";
            features = "ci,docs,hooks,shell-worker,automerge,secrets,git-bridge,deploy";
          });

        # aspen-cli (features: forge,ci,secrets,automerge)
        u2nCliWorkspace = unit2nix.lib.${system}.buildFromUnitGraphAuto (u2nAutoCommon
          // {
            package = "aspen-cli";
            features = "forge,ci,secrets,automerge";
          });

        # git-remote-aspen (features: git-bridge)
        u2nGitRemoteWorkspace = unit2nix.lib.${system}.buildFromUnitGraphAuto (u2nAutoCommon
          // {
            bin = "git-remote-aspen";
            features = "git-bridge";
          });
        aspenGitRemote = u2nGitRemoteWorkspace.workspaceMembers."aspen".build;

        # aspen-sops CLI
        u2nSopsWorkspace = unit2nix.lib.${system}.buildFromUnitGraphAuto (u2nAutoCommon
          // {
            package = "aspen-sops";
          });
        aspenSops = u2nSopsWorkspace.workspaceMembers."aspen-sops".build;

        # aspen-sops-install-secrets: drop-in replacement for sops-nix's Go binary
        u2nSopsInstallWorkspace = unit2nix.lib.${system}.buildFromUnitGraphAuto (u2nAutoCommon
          // {
            package = "aspen-sops-install-secrets";
          });
        aspenSopsInstallSecrets = u2nSopsInstallWorkspace.workspaceMembers."aspen-sops-install-secrets".build;

        bins = let
          bin = {
            name,
            features ? [],
          }:
            craneLib.buildPackage (
              nodeCommonArgs
              // {
                inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                cargoExtraArgs =
                  "--bin ${name}"
                  + lib.optionalString (features != []) " --features ${lib.concatStringsSep "," features}";
                doCheck = false;
              }
            );

          # aspen-tui extracted to ~/git/aspen-tui

          # aspen-cli extracted to ~/git/aspen-cli - builds commented out
          # Build aspen-cli from its own crate
          # aspen-cli-crate = craneLib.buildPackage (
          #   cliCommonArgs
          #   // {
          #     inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
          #     cargoExtraArgs = "--package aspen-cli --bin aspen-cli";
          #     doCheck = false;
          #   }
          # );

          # Build aspen-cli with forge features (push, signed tags, export-key)
          aspen-cli-forge-crate = craneLib.buildPackage (
            cliCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features forge";
              doCheck = false;
            }
          );

          # Build aspen-cli with secrets features (KV, Transit, PKI)
          # Must include ci,automerge to match node's aspen-client-api feature set,
          # otherwise postcard enum discriminants are misaligned and responses
          # deserialize to wrong variants.
          aspen-cli-secrets-crate = craneLib.buildPackage (
            cliCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features secrets,ci,automerge";
              doCheck = false;
            }
          );

          # Build aspen-cli with plugin management features
          # Must include ci,automerge to match node's aspen-client-api enum layout
          # (postcard uses variant indices; cfg-gated variants shift indices)
          aspen-cli-plugins-crate = craneLib.buildPackage (
            cliCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features plugins-rpc,ci,automerge";
              doCheck = false;
            }
          );

          # Build aspen-cli with full features for comprehensive testing
          # Must include ci to match node's aspen-client-api enum layout
          aspen-cli-full-crate = craneLib.buildPackage (
            cliCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features automerge,sql,ci";
              doCheck = false;
            }
          );

          # Build aspen-cli with CI features (CI pipelines + Nix cache)
          aspen-cli-ci-crate = craneLib.buildPackage (
            cliCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features ci";
              doCheck = false;
            }
          );

          # Build aspen-cli with proxy features (TCP tunnel + HTTP forward proxy)
          aspen-cli-proxy-crate = craneLib.buildPackage (
            cliCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features proxy";
              doCheck = false;
            }
          );

          # Build aspen-node with proxy support (default features + proxy)
          aspen-node-proxy = bin {
            name = "aspen-node";
            features = ["ci" "docs" "hooks" "shell-worker" "automerge" "secrets" "proxy"];
          };

          # Build aspen-node with WASM plugin support (plugins-rpc)
          # Uses pre-built hyperlight wasm_runtime to avoid nested cargo build
          aspen-node-plugins = craneLib.buildPackage (
            pluginsCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
              cargoExtraArgs = "--bin aspen-node --features ci,docs,hooks,shell-worker,automerge,secrets,plugins-rpc";
              doCheck = false;
            }
          );

          # aspen-ci extracted to ~/git/aspen-ci

          # aspen-verus-metrics moved to ~/git/aspen-verus-metrics repo

          bins =
            builtins.listToAttrs (
              map ({name, ...} @ package: lib.nameValuePair name (bin package)) [
                {
                  name = "aspen-node";
                  # Required features: jobs, docs, blob, hooks (see Cargo.toml required-features)
                  # ci: Includes jobs+blob via ci-basic, adds SNIX for artifact upload
                  # docs: iroh-docs CRDT document operations
                  # hooks: Event-driven automation (requires jobs)
                  # shell-worker: Execute shell commands for CI shell jobs
                  # Note: "plugins" excluded — hyperlight-wasm build.rs needs network access
                  # which is unavailable in Nix's sandbox.
                  features = ["ci" "docs" "hooks" "shell-worker" "automerge" "secrets"];
                }
                {
                  name = "git-remote-aspen";
                  features = ["git-bridge"];
                }
              ]
            )
            // {
              # aspen-tui extracted to ~/git/aspen-tui
              # aspen-cli: unit2nix build (features: forge,ci,secrets,automerge)
              # All variants point to the same binary since the build plan
              # includes all commonly-needed features.
              aspen-cli = aspenCli;
              aspen-cli-forge = aspenCli;
              aspen-cli-plugins = aspenCli; # plugins-rpc feature not in unit2nix plan; use full- variant for plugin mgmt
              aspen-cli-secrets = aspenCli;
              aspen-cli-full = aspenCli;
              aspen-cli-ci = aspenCli;
              aspen-cli-proxy = aspenCli; # proxy feature not in unit2nix plan; use full- variant for proxy
              # git-remote-aspen: unit2nix build (features: git-bridge)
              git-remote-aspen = aspenGitRemote;
              inherit aspen-node-proxy aspen-node-plugins;
              # aspen-ci-agent extracted to ~/git/aspen-ci
              inherit hyperlight-wasm-runtime;

              # Node with VM CI executor (Cloud Hypervisor) — pure eval build.
              # Uses ciSrc (stubbed aspen-wasm-plugin) so no --impure required.
              # Used by nix run .#dogfood-local-vmci.
              aspen-node-vmci = craneLib.buildPackage (
                ciCommonArgs
                // {
                  inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                  cargoExtraArgs = "--bin aspen-node --features ci,ci-vm-executor,docs,hooks,shell-worker,automerge,secrets,git-bridge";
                  doCheck = false;
                }
              );
            }
            # CI-path builds for VM integration tests (no plugins).
            # Uses ciSrc path which handles snix deps correctly.
            // {
              ci-aspen-node = ciVmTestBin {
                name = "aspen-node";
                features = ["ci" "docs" "hooks" "shell-worker" "automerge" "secrets" "net"];
              };
              # CI node with snix-build for native in-process builds.
              # Pure eval (no --impure), uses ciSrc which keeps snix as real git deps.
              ci-aspen-node-snix-build = craneLib.buildPackage (
                ciCommonArgs
                // {
                  inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                  cargoExtraArgs = "--bin aspen-node --features ci,docs,hooks,shell-worker,automerge,secrets,git-bridge,deploy,snix,snix-build";
                  doCheck = false;
                  PROTO_ROOT = "${snix-src}";
                  SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox-sandbox-shell}/bin/busybox";
                  postInstall = ''
                    mkdir -p $out/share
                    echo "${pkgs.busybox}/bin/sh" > $out/share/sandbox-shell-path
                  '';
                }
              );
              ci-aspen-cli = ciVmTestCliBin [];
              ci-git-remote-aspen = ciVmTestBin {
                name = "git-remote-aspen";
                features = ["git-bridge"];
              };
              # Nix cache gateway built from ciSrc (no plugins dependency).
              ci-aspen-nix-cache-gateway = craneLib.buildPackage (
                ciCommonArgs
                // {
                  pname = "aspen-nix-cache-gateway";
                  version = "0.1.0";
                  cargoExtraArgs = "-p aspen-nix-cache-gateway --bin aspen-nix-cache-gateway";
                  doCheck = false;
                }
              );
              # Nix cache gateway with HTTP/3 over iroh QUIC support.
              ci-aspen-nix-cache-gateway-h3 = craneLib.buildPackage (
                ciCommonArgs
                // {
                  pname = "aspen-nix-cache-gateway";
                  version = "0.1.0";
                  cargoExtraArgs = "-p aspen-nix-cache-gateway --bin aspen-nix-cache-gateway --features h3-serving";
                  doCheck = false;
                }
              );
              # TCP-to-iroh-h3 reverse proxy for bridging HTTP/1.1 clients to H3 endpoints.
              ci-aspen-h3-proxy = craneLib.buildPackage (
                ciCommonArgs
                // {
                  pname = "aspen-h3-proxy";
                  version = "0.1.0";
                  cargoExtraArgs = "-p aspen-h3-proxy --bin aspen-h3-proxy";
                  doCheck = false;
                }
              );
            }
            # CI-path builds with plugins (requires --impure for aspen-wasm-plugin).
            # Uses ciSrc with the stub replaced by real aspen-wasm-plugin source.
            // lib.optionalAttrs hasExternalRepos (let
              ciPluginsSrc = pkgs.runCommand "aspen-ci-plugins-src" {} ''
                cp -r ${ciSrc} $out
                chmod -R u+w $out
                # Replace aspen-wasm-plugin stub with real source
                rm -rf $out/aspen-wasm-plugin
                cp -r ${wasmPluginRepo} $out/aspen-wasm-plugin
                chmod -R u+w $out
              '';
              ciPluginsBasicArgs =
                basicArgs
                // {
                  src = ciPluginsSrc;
                  postUnpack = ''sourceRoot="$sourceRoot/aspen"'';
                  cargoToml = ./Cargo.toml;
                  cargoLock = ciPluginsSrc + "/aspen/Cargo.lock";
                  cargoExtraArgs = "";
                };
              ciPluginsCargoVendorDir = patchVendorForHyperlight (craneLib.vendorCargoDeps {
                src = ciPluginsSrc + "/aspen";
                overrideVendorGitCheckout = ps: drv: let
                  isSnixRepo =
                    builtins.any (
                      p:
                        builtins.isString (p.source or null)
                        && lib.hasPrefix "git+https://git.snix.dev/snix/snix.git" (p.source or "")
                    )
                    ps;
                in
                  if isSnixRepo
                  then ensureGitCheckoutLock (drv.overrideAttrs (_old: {src = snix-src;}))
                  else ensureGitCheckoutLock drv;
              });
              ciPluginsCargoArtifacts = craneLib.buildDepsOnly (
                ciPluginsBasicArgs
                // {
                  cargoVendorDir = ciPluginsCargoVendorDir;
                  HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
                  PROTO_ROOT = "${snix-src}";
                  SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox-sandbox-shell}/bin/busybox";
                  pnameSuffix = "-ci-plugins-deps";
                  cargoExtraArgs = "--features ci,ci-vm-executor,docs,hooks,shell-worker,automerge,secrets,plugins-rpc,forge,git-bridge,blob,net,deploy,proxy,snix,snix-build";
                }
              );
              ciPluginsCommonArgs =
                ciPluginsBasicArgs
                // {
                  cargoArtifacts = ciPluginsCargoArtifacts;
                  cargoVendorDir = ciPluginsCargoVendorDir;
                  HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
                  nativeBuildInputs =
                    basicArgs.nativeBuildInputs
                    ++ (with pkgs; [autoPatchelfHook]);
                  buildInputs =
                    basicArgs.buildInputs
                    ++ (lib.optionals pkgs.stdenv.buildPlatform.isDarwin (
                      with pkgs; [darwin.apple_sdk.frameworks.Security]
                    ));
                };
              ciPluginsBin = {
                name,
                features ? [],
              }:
                craneLib.buildPackage (
                  ciPluginsCommonArgs
                  // {
                    inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                    cargoExtraArgs =
                      "--bin ${name}"
                      + lib.optionalString (features != []) " --features ${lib.concatStringsSep "," features}";
                    doCheck = false;
                  }
                );
              ciPluginsCliBin = features:
                craneLib.buildPackage (
                  ciPluginsCommonArgs
                  // {
                    pname = "aspen-cli";
                    version = "0.1.0";
                    cargoExtraArgs =
                      "-p aspen-cli --bin aspen-cli"
                      + lib.optionalString (features != []) " --features ${lib.concatStringsSep "," features}";
                    doCheck = false;
                  }
                );
            in {
              ci-aspen-node-plugins = craneLib.buildPackage (
                ciPluginsCommonArgs
                // {
                  inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                  cargoExtraArgs = "--bin aspen-node --features ci,ci-vm-executor,docs,hooks,shell-worker,automerge,secrets,plugins-rpc,forge,git-bridge,blob,net,deploy,snix,snix-build";
                  doCheck = false;
                  PROTO_ROOT = "${snix-src}";
                  SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox-sandbox-shell}/bin/busybox";
                  postInstall = ''
                    mkdir -p $out/share
                    echo "${pkgs.busybox}/bin/sh" > $out/share/sandbox-shell-path
                  '';
                }
              );
              ci-aspen-cli-e2e = ciPluginsCliBin ["ci" "forge"];
              ci-aspen-cli-plugins = ciPluginsCliBin ["plugins-rpc" "ci" "automerge"];
            })
            # Full-source builds for VM integration tests (legacy, requires --impure).
            # These have IFD caching issues — prefer ci-* variants above.
            // lib.optionalAttrs hasExternalRepos (let
              # Shared snix vendor dir for full-source builds with snix-build.
              fullSnixVendorDir = patchVendorForHyperlight (craneLib.vendorCargoDeps {
                src = fullSrcWithSnix + "/aspen";
                overrideVendorGitCheckout = ps: drv: let
                  isSnixRepo =
                    builtins.any (
                      p:
                        builtins.isString (p.source or null)
                        && lib.hasPrefix "git+https://git.snix.dev/snix/snix.git" (p.source or "")
                    )
                    ps;
                in
                  if isSnixRepo
                  then ensureGitCheckoutLock (drv.overrideAttrs (_old: {src = snix-src;}))
                  else ensureGitCheckoutLock drv;
              });
              # Node with WASM plugin runtime + snix-build for native in-process builds.
              # Uses bubblewrap/OCI sandbox via snix-build's BuildService instead of
              # shelling out to `nix build`. Falls back to subprocess at runtime if needed.
              fullAspenNodePlugins = craneLib.buildPackage (
                fullBasicArgs
                // {
                  inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                  src = fullSrcWithSnix;
                  cargoArtifacts = fullPluginsCargoArtifacts;
                  cargoVendorDir = fullSnixVendorDir;
                  cargoExtraArgs = "--bin aspen-node --features ci,ci-vm-executor,docs,hooks,shell-worker,automerge,secrets,plugins-rpc,forge,git-bridge,blob,net,deploy,snix,snix-build";
                  doCheck = false;
                  HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
                  PROTO_ROOT = "${snix-src}";
                  SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox-sandbox-shell}/bin/busybox";
                  postInstall = ''
                    mkdir -p $out/share
                    echo "${pkgs.busybox}/bin/sh" > $out/share/sandbox-shell-path
                  '';
                  nativeBuildInputs =
                    basicArgs.nativeBuildInputs
                    ++ (with pkgs; [autoPatchelfHook]);
                  buildInputs =
                    basicArgs.buildInputs
                    ++ (lib.optionals pkgs.stdenv.buildPlatform.isDarwin (
                      with pkgs; [darwin.apple_sdk.frameworks.Security]
                    ));
                }
              );
            in {
              full-aspen-node = fullBin {
                name = "aspen-node";
                features = ["ci" "docs" "hooks" "shell-worker" "automerge" "secrets" "net"];
              };
              full-aspen-node-proxy = fullBin {
                name = "aspen-node";
                features = ["ci" "docs" "hooks" "shell-worker" "automerge" "secrets" "proxy"];
              };
              # Node with VM CI executor (Cloud Hypervisor). Used by dogfood-local-vmci.
              # NOTE: Also built without hasExternalRepos below (as aspen-node-vmci)
              full-aspen-node-vmci = fullBin {
                name = "aspen-node";
                features = ["ci" "ci-vm-executor" "docs" "hooks" "shell-worker" "automerge" "secrets" "git-bridge"];
              };
              full-aspen-node-plugins = fullAspenNodePlugins;
              # Legacy alias — full-aspen-node-plugins now includes snix.
              full-aspen-node-plugins-snix = fullAspenNodePlugins;
              # Backward-compat alias: snix-build without deploy/ci-vm-executor.
              # Used by snix-native-build-test which doesn't need those features.
              full-aspen-node-plugins-snix-build = craneLib.buildPackage (
                fullBasicArgs
                // {
                  inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                  src = fullSrcWithSnix;
                  cargoArtifacts = fullPluginsCargoArtifacts;
                  cargoVendorDir = fullSnixVendorDir;
                  cargoExtraArgs = "--bin aspen-node --features ci,docs,hooks,shell-worker,automerge,secrets,plugins-rpc,forge,git-bridge,blob,net,snix,snix-build";
                  doCheck = false;
                  HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
                  PROTO_ROOT = "${snix-src}";
                  SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox-sandbox-shell}/bin/busybox";
                  postInstall = ''
                    mkdir -p $out/share
                    echo "${pkgs.busybox}/bin/sh" > $out/share/sandbox-shell-path
                  '';
                  nativeBuildInputs =
                    basicArgs.nativeBuildInputs
                    ++ (with pkgs; [autoPatchelfHook]);
                  buildInputs =
                    basicArgs.buildInputs
                    ++ (lib.optionals pkgs.stdenv.buildPlatform.isDarwin (
                      with pkgs; [darwin.apple_sdk.frameworks.Security]
                    ));
                }
              );
              full-aspen-net-daemon = craneLib.buildPackage (
                fullCommonArgs
                // {
                  inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                  cargoExtraArgs = "--package aspen-net --bin aspen-net";
                  doCheck = false;
                }
              );
              full-aspen-cli = fullCliBin [];
              full-aspen-cli-forge = fullCliBin ["forge"];
              full-aspen-cli-plugins = fullCliBin ["plugins-rpc" "ci" "automerge"];
              full-aspen-cli-ci = fullCliBin ["ci"];
              full-aspen-cli-e2e = fullCliBin ["ci" "forge"];
              full-aspen-cli-secrets = fullCliBin ["secrets" "ci"];
              full-aspen-cli-proxy = fullCliBin ["proxy"];
              full-git-remote-aspen = fullBin {
                name = "git-remote-aspen";
                features = ["git-bridge"];
              };
              # gRPC bridge: snix-store CLI ↔ Aspen distributed storage
              full-aspen-snix-bridge = craneLib.buildPackage (
                fullCommonArgs
                // {
                  pname = "aspen-snix-bridge";
                  version = "0.1.0";
                  cargoExtraArgs = "-p aspen-snix-bridge --bin aspen-snix-bridge --features snix-daemon";
                  doCheck = false;
                }
              );
              # HTTP gateway: Nix binary cache protocol ↔ Aspen cluster RPC
              full-aspen-nix-cache-gateway = craneLib.buildPackage (
                fullCommonArgs
                // {
                  pname = "aspen-nix-cache-gateway";
                  version = "0.1.0";
                  cargoExtraArgs = "-p aspen-nix-cache-gateway --bin aspen-nix-cache-gateway";
                  doCheck = false;
                }
              );
            });
        in
          bins
          // rec {
            default = aspenNode;

            # Development builds with incremental compilation enabled
            # Use these for faster iteration during development
            dev-aspen-node = craneLib.buildPackage (
              devArgs
              // {
                inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                cargoExtraArgs = "--bin aspen-node --features full";
                doCheck = false;
              }
            );

            # dev-aspen-tui extracted to ~/git/aspen-tui

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
              # Verify ciSrc doesn't contain crates with unvendored git deps.
              # These would fail in the nix sandbox (no network access).
              # Catches the case where a new crate is added to the workspace
              # but not excluded from ciSrc.
              ciSrc-no-unvendored-git-deps = pkgs.runCommand "ciSrc-no-unvendored-git-deps" {} ''
                cargo_toml="${ciSrc}/aspen/Cargo.toml"
                cargo_lock="${ciSrc}/aspen/Cargo.lock"

                # Crates that must NOT appear as workspace members or deps
                # in ciSrc Cargo.toml (their git deps aren't vendored)
                for crate in aspen-testing-patchbay aspen-tui; do
                  # Match workspace member entries and [workspace.dependencies] lines,
                  # skip comments (lines starting with #)
                  if grep -v '^\s*#' "$cargo_toml" | grep -q "$crate" 2>/dev/null; then
                    echo "ERROR: $crate still referenced in ciSrc Cargo.toml:"
                    grep -v '^\s*#' "$cargo_toml" | grep "$crate"
                    echo "Add to ciSrc stripping in flake.nix"
                    exit 1
                  fi
                done

                # No git+ source lines should remain in Cargo.lock except
                # snix.dev and tvlfyi — these are vendored via overrideVendorGitCheckout
                if grep '^source = "git+' "$cargo_lock" | grep -v -e 'snix\.dev' -e 'tvlfyi' 2>/dev/null | grep -q .; then
                  echo "ERROR: ciSrc Cargo.lock still has unvendored git+ source lines:"
                  grep '^source = "git+' "$cargo_lock" | grep -v -e 'snix\.dev' -e 'tvlfyi'
                  exit 1
                fi

                echo "ciSrc has no unvendored git deps" > $out
              '';

              # Clippy and doc checks using ciCommonArgs (stubbed aspen-wasm-plugin).
              # Works in pure evaluation without external repos.
              clippy = craneLib.cargoClippy (
                ciCommonArgs
                // {
                  # Exclude crates that can't compile with stubbed/missing deps in ciSrc:
                  # - aspen-testing-patchbay: patchbay git dep stubbed in ciSrc
                  # - aspen-tui: rattoolkit git dep stubbed in ciSrc
                  # snix crates (aspen-castore, aspen-snix, aspen-snix-bridge) now
                  # compile in CI — snix deps are real (not stubbed), vendored via
                  # overrideVendorGitCheckout with snix-src flake input.
                  cargoClippyExtraArgs = "--workspace --exclude aspen-nix-cache-gateway --exclude aspen-testing-patchbay --exclude aspen-tui -- -D warnings";
                }
              );

              doc = let
                mainCrates = [
                  "aspen"
                  "aspen-auth"
                  "aspen-blob"
                  "aspen-client"
                  "aspen-cluster"
                  "aspen-core"
                ];
                pkgArgs = lib.concatMapStringsSep " " (c: "-p ${c}") mainCrates;
              in
                craneLib.cargoDoc (
                  ciCommonArgs
                  // {
                    cargoDocExtraArgs = "${pkgArgs} --no-deps";
                    RUSTDOCFLAGS = "-D warnings";
                  }
                );

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

              # Deny check stubbed out — requires extracted sibling repo sources
              # Run locally: cargo deny check
              deny = pkgs.runCommand "aspen-deny-stub" {} ''
                echo "SKIPPED: cargo deny requires extracted sibling repo sources"
                echo "Run locally: cargo deny check"
                touch $out
              '';

              audit = craneLib.cargoAudit {
                inherit src;
                # Patch advisory-db: strip CVSS 4.0 lines which cargo-audit can't parse
                advisory-db = pkgs.runCommand "advisory-db-patched" {} ''
                  cp -r ${advisory-db} $out
                  chmod -R u+w $out
                  find $out -name '*.md' -exec \
                    ${pkgs.gnused}/bin/sed -i '/^cvss = "CVSS:4\./d' {} +
                '';
                # RUSTSEC-2023-0071: rsa crate timing sidechannel — no fix available,
                # transitive dep via ssh-key → aspen-forge
                cargoAuditExtraArgs = "--ignore RUSTSEC-2023-0071";
              };

              # Verus formal verification check
              # Verifies standalone Verus specifications (Core + Cluster)
              # NOTE: Raft and Transport specs extracted to sibling repos
              verus-check =
                pkgs.runCommand "aspen-verus-check" {
                  nativeBuildInputs = [verus z3_4_12_5];
                } ''
                  export VERUS_Z3_PATH="${z3_4_12_5}/bin/z3"
                  export VERUS_ROOT="${verusRoot}"
                  export LD_LIBRARY_PATH="${verusRustToolchain}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

                  echo "=== Verifying All Aspen Verus Specifications ==="

                  # NOTE: All verus specs extracted to sibling repos:
                  #   Core specs    → ~/git/aspen-core/crates/aspen-core/verus/
                  #   Cluster specs → ~/git/aspen-cluster/crates/aspen-cluster/verus/
                  #   Raft specs    → ~/git/aspen-raft/crates/aspen-raft/verus/
                  #   Transport specs → ~/git/aspen-transport/crates/aspen-transport/verus/
                  # Run locally: cd ~/git/aspen-{core,cluster,raft,transport} && verus ...
                  echo "[1/4] Core specifications — SKIPPED (extracted to aspen-core repo)"
                  echo "[2/4] Raft specifications — SKIPPED (extracted to aspen-raft repo)"
                  echo "[3/4] Cluster specifications — SKIPPED (extracted to aspen-cluster repo)"
                  echo "[4/4] Transport specifications — SKIPPED (extracted to aspen-transport repo)"

                  echo "=== All specifications in sibling repos ==="
                  touch $out
                '';

              # Verus inline ghost code check
              # Verifies that production code with ghost annotations compiles with verus feature
              # NOTE: verus-inline-check disabled — requires aspen-layer (extracted
              # to sibling repo). Run locally: cargo check -p aspen-raft --features verus
              verus-inline-check = pkgs.runCommand "aspen-verus-inline-check" {} ''
                echo "SKIPPED: verus-inline-check requires extracted sibling repos (aspen-layer)"
                echo "Run locally: cargo check -p aspen-raft --features verus"
                touch $out
              '';

              # verus-sync-check: moved to ~/git/aspen-verus-metrics repo
            }
            // {
              # Run quick tests with cargo-nextest (for CI)
              # Uses ciCommonArgs (stubbed aspen-wasm-plugin) — works in pure
              # evaluation without external repos. All workspace crates are real.
              # Uses ci-nix profile which excludes disk-heavy tests that fail in
              # the Nix sandbox (tmpfs has limited space).
              nextest-quick = craneLib.cargoNextest (
                ciCommonArgs
                // {
                  cargoNextestExtraArgs = "-P ci-nix";
                  nativeBuildInputs =
                    ciCommonArgs.nativeBuildInputs
                    ++ [pkgs.cargo-nextest];
                }
              );

              nextest = craneLib.cargoNextest (
                ciCommonArgs
                // {
                  # Full test run in Nix sandbox — uses ci-nix profile to skip
                  # disk-heavy corruption tests that fail on tmpfs.
                  cargoNextestExtraArgs = "-P ci-nix";
                  nativeBuildInputs =
                    ciCommonArgs.nativeBuildInputs
                    ++ [pkgs.cargo-nextest];
                }
              );
              # CI build checks — verify binaries compile with CI features.
              # Uses ciCommonArgs (stubbed git deps, no network needed).
              build-node = craneLib.buildPackage (
                ciCommonArgs
                // {
                  inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                  cargoExtraArgs = "--bin aspen-node --features ci,docs,hooks,shell-worker,automerge,secrets,proxy,forge,git-bridge,blob,sql,net,deploy,federation,global-discovery,jobs,kv-branch,nostr-relay,relay-server,snix,snix-http,snix-daemon,snix-eval,snix-build";
                  doCheck = false;
                }
              );

              build-cli = craneLib.buildPackage (
                ciCommonArgs
                // {
                  inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
                  cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features forge,ci,automerge,sql,secrets,blob,proxy";
                  doCheck = false;
                }
              );

              # SNIX rev drift guard: verify the SNIX_GIT_SOURCE constant in
              # checkout.rs matches the snix-src flake input revision. Prevents
              # silent drift between the hardcoded rev and the actual dependency.
              snix-rev-check = let
                lockfile = builtins.fromJSON (builtins.readFile ./flake.lock);
                snixFlakeRev = lockfile.nodes."snix-src".locked.rev;
              in
                pkgs.runCommand "snix-rev-check" {
                  src = fullRawSrc;
                  inherit snixFlakeRev;
                } ''
                  CHECKOUT_RS="$src/crates/aspen-ci/src/checkout.rs"
                  if [ ! -f "$CHECKOUT_RS" ]; then
                    echo "FAIL: checkout.rs not found at $CHECKOUT_RS"
                    exit 1
                  fi

                  CHECKOUT_REV=$(${pkgs.gnugrep}/bin/grep -oP 'SNIX_GIT_SOURCE.*\?rev=\K[0-9a-f]+' "$CHECKOUT_RS" | head -1)

                  if [ -z "$CHECKOUT_REV" ]; then
                    echo "FAIL: could not extract rev from SNIX_GIT_SOURCE in checkout.rs"
                    exit 1
                  fi

                  echo "Flake snix-src rev:   $snixFlakeRev"
                  echo "checkout.rs rev:      $CHECKOUT_REV"

                  if [ "$CHECKOUT_REV" != "$snixFlakeRev" ]; then
                    echo ""
                    echo "DRIFT DETECTED: SNIX_GIT_SOURCE in checkout.rs ($CHECKOUT_REV)"
                    echo "  does not match snix-src flake input ($snixFlakeRev)"
                    echo ""
                    echo "Fix: update the rev in crates/aspen-ci/src/checkout.rs line ~361"
                    echo "  to match: $snixFlakeRev"
                    exit 1
                  fi

                  echo "PASS: SNIX revs match"
                  touch $out
                '';
            }
            # ── Per-crate unit tests (unit2nix workspace mode) ─────────────
            # Each workspace member gets a checks.test-<name> that compiles
            # and runs its #[test] binaries. Only the changed crate and its
            # reverse deps are rebuilt — deps stay cached.
            // lib.mapAttrs'
            (name: drv: lib.nameValuePair "test-${name}" drv)
            (lib.filterAttrs (
                name: _:
                # Exclude stubs (git deps replaced with empty crates for IFD),
                # crates that unconditionally import from stubs, and vendored
                # openraft (tested upstream).
                  !builtins.elem name [
                    # Stubs themselves (git deps replaced with empty crates)
                    "iroh-h3"
                    "iroh-h3-axum" # depends on iroh-h3 stub
                    "iroh-proxy-utils"
                    "mad-turmoil"
                    "nix-compat"
                    "nix-compat-derive"
                    "patchbay"
                    "snix-build"
                    "snix-castore"
                    "snix-castore-http"
                    "snix-eval"
                    "snix-glue"
                    "snix-serde"
                    "snix-store"
                    "snix-tracing"
                    "nix-daemon"
                    "nar-bridge"
                    # Crates with unconditional deps on stubs
                    "aspen-castore"
                    "aspen-snix"
                    "aspen-snix-bridge" # snix-*
                    "aspen-proxy"
                    "aspen-net" # iroh-proxy-utils
                    "aspen-testing-madsim" # mad-turmoil
                    # nix-compat stub: aspen-cache uses nix_compat::{nixbase32,nar,narinfo,store_path}
                    # which the stub doesn't provide. Only crates with DIRECT nix-compat deps excluded.
                    "aspen-cache"
                    "aspen-nix-cache-gateway"
                    "aspen-ci-executor-nix"
                    "aspen-ci-executor-vm" # unconditional dep on aspen-fuse (needs /dev/fuse)
                    # patchbay stub: dev-deps pull in aspen-testing-patchbay → patchbay
                    "aspen-dag"
                    "aspen-forge"
                    "aspen-testing-patchbay" # depends on patchbay git dep
                    # buildRustCrate ignores required-features — gated tests get compiled
                    "aspen"
                    "aspen-rpc-handlers"
                    # Sandbox-incompatible (needs /dev/fuse or git at runtime)
                    "aspen-fuse"
                    "aspen-ci"
                    # CARGO_BIN_EXE_* not set by buildRustCrate
                    "aspen-sops"
                    # Needs root for mount/chown — tested in NixOS VM tests
                    "aspen-sops-install-secrets"
                    # Vendored (tested upstream)
                    "openraft"
                    "openraft-macros"
                  ]
              )
              u2nTestWorkspace.test.check)
            # ── Real checks (override stubs when aspen-wasm-plugin available) ──
            // lib.optionalAttrs hasExternalRepos {
              clippy = craneLib.cargoClippy (
                fullCommonArgs
                // {
                  # Exclude crates with build issues even in full source:
                  # - aspen-testing-patchbay: patchbay git dep not in fullSrc vendor
                  # - aspen-tui: rattoolkit API breakage (deprecated ratatui methods)
                  cargoClippyExtraArgs = "--workspace --exclude aspen-nix-cache-gateway --exclude aspen-testing-patchbay --exclude aspen-tui -- -D warnings";
                }
              );

              doc = let
                # Only document main workspace crates (sibling repos have pre-existing doc issues)
                mainCrates = [
                  "aspen"
                  "aspen-auth"
                  "aspen-blob"
                  "aspen-client"
                  "aspen-cluster"
                  "aspen-core"
                ];
                pkgArgs = lib.concatMapStringsSep " " (c: "-p ${c}") mainCrates;
              in
                craneLib.cargoDoc (
                  fullCommonArgs
                  // {
                    cargoDocExtraArgs = "${pkgArgs} --no-deps";
                    RUSTDOCFLAGS = "-D warnings";
                  }
                );

              deny = let
                # Patch advisory-db: strip CVSS 4.0 lines which cargo-deny can't parse
                patchedAdvisoryDb = pkgs.runCommand "advisory-db-patched" {} ''
                  cp -r ${advisory-db} $out
                  chmod -R u+w $out
                  find $out -name '*.md' -exec \
                    ${pkgs.gnused}/bin/sed -i '/^cvss = "CVSS:4\./d' {} +
                '';
              in
                craneLib.mkCargoDerivation {
                  src = fullSrc;
                  postUnpack = ''sourceRoot="$sourceRoot/aspen"'';
                  cargoArtifacts = null;
                  cargoVendorDir = fullPluginsCargoVendorDir;
                  pname = "aspen-deny";
                  nativeBuildInputs = [pkgs.cargo-deny rustToolChain];
                  HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
                  buildPhaseCargoCommand = ''
                    # Pre-populate advisory-db in the exact location cargo-deny expects.
                    # Directory name is hash of default URL: https://github.com/RustSec/advisory-db
                    DB_DIR="$CARGO_HOME/advisory-dbs"
                    mkdir -p "$DB_DIR"
                    # cargo-deny expects a git repo. Init one from the advisory-db source.
                    cp -r ${patchedAdvisoryDb} "$DB_DIR/advisory-db-3157b0e258782691"
                    chmod -R u+w "$DB_DIR/advisory-db-3157b0e258782691"
                    (cd "$DB_DIR/advisory-db-3157b0e258782691" && \
                      ${pkgs.git}/bin/git init -q && \
                      ${pkgs.git}/bin/git add -A && \
                      ${pkgs.git}/bin/git -c user.email=nix@build -c user.name=nix commit -q -m "init")
                    # Point deny.toml at local db-path
                    ${pkgs.gnused}/bin/sed -i \
                      '/^\[advisories\]/a db-path = "'"$DB_DIR"'"' \
                      deny.toml
                    cargo deny check --disable-fetch
                  '';
                  installPhase = "touch $out";
                };

              nextest-quick = craneLib.cargoNextest (
                fullCommonArgs
                // {
                  cargoNextestExtraArgs = "-P quick";
                  nativeBuildInputs =
                    fullCommonArgs.nativeBuildInputs
                    ++ [pkgs.cargo-nextest];
                }
              );

              nextest = craneLib.cargoNextest (
                fullCommonArgs
                // {
                  nativeBuildInputs =
                    fullCommonArgs.nativeBuildInputs
                    ++ [pkgs.cargo-nextest];
                }
              );

              verus-inline-check = craneLib.mkCargoDerivation (
                fullCommonArgs
                // {
                  pname = "aspen-verus-inline-check";
                  # verus feature lives in extracted crates (aspen-raft, aspen-coordination)
                  # which are available as deps in fullSrc via ../aspen-raft/ etc.
                  buildPhaseCargoCommand = ''
                    echo "[1/2] Checking aspen-raft with verus feature..."
                    cargo check -p aspen-raft --features verus
                    echo "[2/2] Checking aspen-coordination with verus feature..."
                    cargo check -p aspen-coordination --features verus
                    echo "All verus inline checks passed"
                  '';
                  installPhase = "touch $out";
                }
              );
            }
            // lib.optionalAttrs (system == "x86_64-linux" && hasExternalRepos) {
              # NixOS VM integration tests — builds node/CLI from the workspace.
              # Requires --impure for aspen-wasm-plugin (plugins-rpc feature).
              # Run individually: nix build .#checks.x86_64-linux.forge-cluster-test --impure
              # NixOS VM integration test for the Forge.
              # Spins up a cluster with real networking and exercises
              # every forge CLI command end-to-end.
              # Build: nix build .#checks.x86_64-linux.forge-cluster-test
              forge-cluster-test = import ./nix/tests/forge-cluster.nix {
                inherit pkgs kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli-forge;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gitRemoteAspenPackage = bins.full-git-remote-aspen;
              };

              # Forge DAG sync test: 2-node cluster, git push on node1,
              # git clone from node2 verifying cross-node object sync.
              # Build: nix build .#checks.x86_64-linux.forge-dag-sync-test
              forge-dag-sync-test = import ./nix/tests/forge-dag-sync.nix {
                inherit pkgs kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli-forge;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gitRemoteAspenPackage = bins.full-git-remote-aspen;
              };

              # Multi-node cluster test: 3-node Raft consensus, replication,
              # leader failover, cross-node operations, and node rejoin.
              # Build: nix build .#checks.x86_64-linux.multi-node-cluster-test
              multi-node-cluster-test = import ./nix/tests/multi-node-cluster.nix {
                inherit pkgs kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.ci-aspen-node-plugins;
                aspenCliPackage = bins.ci-aspen-cli-e2e;
                aspenCliPlugins = bins.ci-aspen-cli-plugins;
              };

              # Alert failover test: alert rule fires, leadership transfer,
              # alert state survives on new leader, periodic evaluator picks up,
              # alert resolves when metrics drop.
              # Build: nix build .#checks.x86_64-linux.alert-failover-test
              alert-failover-test = import ./nix/tests/alert-failover.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # Multi-node KV test: write/read replication, CAS across nodes,
              # batch write replication, scan consistency, NOT_LEADER routing,
              # delete propagation, large value replication, failover survival.
              # Build: nix build .#checks.x86_64-linux.multi-node-kv-test
              multi-node-kv-test = import ./nix/tests/multi-node-kv.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # Multi-node coordination test: distributed lock exclusion,
              # counter linearizability, semaphore capacity, RW lock guarantees,
              # queue cross-node ops, failover survival.
              # Build: nix build .#checks.x86_64-linux.multi-node-coordination-test --impure
              multi-node-coordination-test = import ./nix/tests/multi-node-coordination.nix {
                inherit pkgs coordinationPluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
              };

              # Multi-node blob test: cross-node blob retrieval, blobs from
              # different nodes, replication status, large blob replication,
              # protection across nodes, repair cycle, failover survival.
              # Build: nix build .#checks.x86_64-linux.multi-node-blob-test
              multi-node-blob-test = import ./nix/tests/multi-node-blob.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # KV operations test: CRUD, CAS/CAD, scan/pagination, batch ops,
              # binary data, large values, special characters.
              # Build: nix build .#checks.x86_64-linux.kv-operations-test
              kv-operations-test = import ./nix/tests/kv-operations.nix {
                inherit pkgs;
                aspenNodePackage = bins.ci-aspen-node;
                aspenCliPackage = bins.ci-aspen-cli;
              };

              # Commit DAG test: chain-hashed commits, system prefix storage,
              # commit chain simulation, atomic batches, prefix isolation.
              # Build: nix build .#checks.x86_64-linux.commit-dag-test
              commit-dag-test = import ./nix/tests/commit-dag.nix {
                inherit pkgs;
                aspenNodePackage = bins.ci-aspen-node;
                aspenCliPackage = bins.ci-aspen-cli;
              };

              # FUSE mount test: write, read, mkdir, delete, large file (100KB).
              # Build: nix build .#checks.x86_64-linux.fuse-operations-test
              fuse-operations-test = import ./nix/tests/fuse-operations.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
                inherit (self.packages.${system}) aspen-fuse-vm-test;
              };

              # Lazy fetch test: open() doesn't prefetch, read() fetches on demand,
              # access stats logged on unmount.
              # Build: nix build .#checks.x86_64-linux.fuse-lazy-fetch-test
              fuse-lazy-fetch-test = import ./nix/tests/fuse-lazy-fetch.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
                inherit (self.packages.${system}) aspen-fuse-vm-test;
              };

              # Execution cache test: FUSE mount + BLAKE3 content hashing +
              # cache entry storage/lookup via KV _exec_cache: prefix.
              # Build: nix build .#checks.x86_64-linux.exec-cache-test
              exec-cache-test = import ./nix/tests/exec-cache.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
                inherit (self.packages.${system}) aspen-fuse-vm-test;
              };

              # Blob operations test: add, get, has, list, protect/unprotect,
              # status, replication-status, delete, repair-cycle.
              # Build: nix build .#checks.x86_64-linux.blob-operations-test
              blob-operations-test = import ./nix/tests/blob-operations.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # Coordination primitives test: locks, RW locks, counters,
              # sequences, semaphores, barriers, queues, leases.
              # Build: nix build .#checks.x86_64-linux.coordination-primitives-test --impure
              coordination-primitives-test = import ./nix/tests/coordination-primitives.nix {
                inherit pkgs coordinationPluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
              };

              # Hooks + service registry test: hook list/metrics/trigger,
              # service register/discover/heartbeat/deregister.
              # Build: nix build .#checks.x86_64-linux.hooks-services-test --impure
              hooks-services-test = import ./nix/tests/hooks-services.nix {
                inherit pkgs serviceRegistryPluginWasm hooksPluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
              };

              # Rate limiter + verify test: token bucket rate limiting,
              # KV/blob storage verification.
              # Build: nix build .#checks.x86_64-linux.ratelimit-verify-test --impure
              ratelimit-verify-test = import ./nix/tests/ratelimit-verify.nix {
                inherit pkgs coordinationPluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
              };

              # Cluster management, docs namespace, peer, and verify test:
              # cluster (status, health, metrics, ticket, prometheus);
              # docs CRDT (set, get, delete, list); peer list;
              # verify (kv, blob, docs, all).
              # Build: nix build .#checks.x86_64-linux.cluster-docs-peer-test
              cluster-docs-peer-test = import ./nix/tests/cluster-docs-peer.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # Job queue and index management test: job submit, status,
              # list, cancel, purge; index list, show (built-in indexes).
              # Build: nix build .#checks.x86_64-linux.job-index-test
              job-index-test = import ./nix/tests/job-index.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # Worker PSI health: /proc/pressure data flows through heartbeats
              # into __worker_stats: KV entries with pressure + disk free fields.
              # Build: nix build .#checks.x86_64-linux.worker-psi-health-test
              worker-psi-health-test = import ./nix/tests/worker-psi-health.nix {
                inherit pkgs;
                aspenNodePackage = bins.ci-aspen-node;
                aspenCliPackage = bins.ci-aspen-cli;
              };

              # CI failure cache: failing nix build is cached, second run skips build.
              # Build: nix build .#checks.x86_64-linux.ci-failure-cache-test
              ci-failure-cache-test = import ./nix/tests/ci-failure-cache.nix {
                inherit pkgs kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.ci-aspen-node-plugins;
                aspenCliPackage = bins.ci-aspen-cli-e2e;
                aspenCliPlugins = bins.ci-aspen-cli-plugins;
                gitRemoteAspenPackage = bins.ci-git-remote-aspen;
              };

              # NixBuildSupervisor timeout: hanging nix build killed after timeout,
              # worker recovers and processes subsequent jobs.
              # Build: nix build .#checks.x86_64-linux.nix-build-supervisor-test
              nix-build-supervisor-test = import ./nix/tests/nix-build-supervisor.nix {
                inherit pkgs kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.ci-aspen-node-plugins;
                aspenCliPackage = bins.ci-aspen-cli-e2e;
                aspenCliPlugins = bins.ci-aspen-cli-plugins;
                gitRemoteAspenPackage = bins.ci-git-remote-aspen;
              };

              # Deploy statefulness: deploy executor writes lifecycle state
              # to _deploy:state:{id}:metadata and per-node KV entries.
              # Build: nix build .#checks.x86_64-linux.deploy-statefulness-test --option sandbox false
              deploy-statefulness-test = import ./nix/tests/deploy-statefulness.nix {
                inherit pkgs lib kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.ci-aspen-node-plugins;
                aspenCliPackage = bins.ci-aspen-cli-e2e;
                aspenCliPlugins = bins.ci-aspen-cli-plugins;
                gitRemoteAspenPackage = bins.ci-git-remote-aspen;
                nixpkgsFlake = nixpkgs;
              };

              # Automerge CRDT + SQL query test: create/get/list/delete
              # CRDT documents, execute SQL queries against KV store.
              # Build: nix build .#checks.x86_64-linux.automerge-sql-test --impure
              automerge-sql-test = import ./nix/tests/automerge-sql.nix {
                inherit pkgs automergePluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
              };

              # Plugin CLI test: install, list, info, enable, disable, remove,
              # reload, manifest-based install, flag overrides, resource limits,
              # KV prefix configuration, reinstall/overwrite.
              # Build: nix build .#checks.x86_64-linux.plugin-cli-test
              plugin-cli-test = import ./nix/tests/plugin-cli.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli-plugins;
              };

              # Secrets engine test: KV v2 (put/get/versions/delete/undelete/destroy),
              # Transit (create-key/encrypt/decrypt/sign/verify/rotate).
              # Build: nix build .#checks.x86_64-linux.secrets-engine-test --impure
              secrets-engine-test = import ./nix/tests/secrets-engine.nix {
                inherit pkgs secretsPluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli-secrets;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
              };

              # SOPS Transit integration test: Transit data key generation,
              # encrypt/decrypt round-trip, key rotation, rewrap, envelope flow.
              # Build: nix build .#checks.x86_64-linux.sops-transit-test --impure
              sops-transit-test = import ./nix/tests/sops-transit.nix {
                inherit pkgs secretsPluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli-secrets;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                aspenSopsPackage = aspenSops;
              };

              # sops-install-secrets integration test: age-only decrypt at boot,
              # template rendering, permissions, generation pruning.
              # Build: nix build .#checks.x86_64-linux.sops-install-secrets-test --impure
              sops-install-secrets-test = import ./nix/tests/sops-install-secrets.nix {
                inherit pkgs;
                aspenSopsPackage = aspenSops;
                aspenSopsInstallSecretsPackage = aspenSopsInstallSecrets;
              };

              # CI pipeline and Nix binary cache test: CI lifecycle
              # (run, status, list, cancel, watch, unwatch, output) and
              # cache operations (stats, query, download).
              # Build: nix build .#checks.x86_64-linux.ci-cache-test
              ci-cache-test = import ./nix/tests/ci-cache.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli-ci;
              };

              # Forge-CI integration test: end-to-end pipeline from
              # CI commands through ref-status tracking.
              # Build: nix build .#checks.x86_64-linux.forge-ci-integration-test
              forge-ci-integration-test = import ./nix/tests/forge-ci-integration.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli-ci;
              };

              # Build: nix build .#checks.x86_64-linux.forge-ci-commit-status-test
              forge-ci-commit-status-test = import ./nix/tests/forge-ci-commit-status.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli-ci;
              };

              # SNIX store test: full SNIX storage stack end-to-end.
              # Blob storage, directory metadata (snix:dir:), path info
              # (snix:pathinfo:), Nix binary cache (query, stats, download),
              # namespace isolation, and simulated CI build artifact flow.
              # Build: nix build .#checks.x86_64-linux.snix-store-test
              snix-store-test = import ./nix/tests/snix-store.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # Nix cache gateway test: HTTP binary cache protocol bridge.
              # Starts cluster + gateway, verifies nix-cache-info, narinfo
              # signing, 404/400 error handling, and CLI public-key command.
              # Build: nix build .#checks.x86_64-linux.nix-cache-gateway-test
              nix-cache-gateway-test = import ./nix/tests/nix-cache-gateway.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli-ci;
                gatewayPackage = bins.full-aspen-nix-cache-gateway;
              };

              # Nix cache via HTTP/3 over iroh QUIC: gateway --h3 + h3-proxy bridge.
              # Verifies end-to-end H3 path: nix client → h3-proxy → gateway (iroh QUIC) → cluster.
              # Build: nix build .#checks.x86_64-linux.nix-cache-h3-test
              nix-cache-h3-test = import ./nix/tests/nix-cache-h3-test.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli-ci;
                gatewayH3Package = bins.ci-aspen-nix-cache-gateway-h3;
                h3ProxyPackage = bins.ci-aspen-h3-proxy;
              };

              # End-to-end push→build→cache test: full self-hosting pipeline.
              # Creates forge repo, pushes code with CI config via git-remote-aspen,
              # triggers CI pipeline (shell stage), waits for completion,
              # then verifies cache gateway serves the cache info endpoint.
              # Build: nix build .#checks.x86_64-linux.e2e-push-build-cache-test --impure
              e2e-push-build-cache-test = import ./nix/tests/e2e-push-build-cache.nix {
                inherit pkgs kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli-e2e;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gatewayPackage = bins.full-aspen-nix-cache-gateway;
                gitRemoteAspenPackage = bins.full-git-remote-aspen;
              };

              # CI Nix build test: push a Nix flake → CI triggers nix build → verify output.
              # Proves NixBuildWorker can execute `nix build` on flake code pushed to Forge.
              # Build: nix build .#checks.x86_64-linux.ci-nix-build-test --impure
              ci-nix-build-test = import ./nix/tests/ci-nix-build.nix {
                inherit pkgs kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli-e2e;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gitRemoteAspenPackage = bins.full-git-remote-aspen;
              };

              # Dogfood test: push Aspen's own source to Forge and BUILD it.
              # Pushes the full 80-crate workspace, auto-triggers CI pipeline,
              # validates checkout integrity, then compiles aspen-constants
              # Pushes Aspen source to Forge, runs NixBuildWorker with `type = 'nix` jobs.
              # Build: nix build .#checks.x86_64-linux.ci-dogfood-test --impure --option sandbox false
              # Note: --option sandbox false gives the VM internet access via QEMU user-mode NAT
              # so inner nix builds can resolve flake inputs (nixpkgs) from the registry.
              ci-dogfood-test = import ./nix/tests/ci-dogfood.nix {
                inherit pkgs lib kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli-e2e;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gitRemoteAspenPackage = bins.full-git-remote-aspen;
                nixpkgsFlake = nixpkgs;
              };

              # Deploy pipeline test: push flake with deploy stage, CI builds, deploy runs.
              # Proves the Forge → CI → Build → Deploy loop works.
              # Build: nix build .#checks.x86_64-linux.ci-dogfood-deploy-test --impure --option sandbox false
              ci-dogfood-deploy-test = import ./nix/tests/ci-dogfood-deploy.nix {
                inherit pkgs lib kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli-e2e;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gitRemoteAspenPackage = bins.full-git-remote-aspen;
                nixpkgsFlake = nixpkgs;
              };

              # Multi-node deploy pipeline test: 3-node cluster, Forge push, CI build,
              # deploy stage exercises DeploymentCoordinator rolling upgrade path.
              # Build: nix build .#checks.x86_64-linux.ci-dogfood-deploy-multinode-test --impure --option sandbox false
              ci-dogfood-deploy-multinode-test = import ./nix/tests/ci-dogfood-deploy-multinode.nix {
                inherit pkgs lib kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli-e2e;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gitRemoteAspenPackage = bins.full-git-remote-aspen;
                nixpkgsFlake = nixpkgs;
              };

              # Self-build test: push Aspen Rust code to Forge, CI compiles it with rustc.
              # Proves the self-hosting loop: Aspen builds its own Rust code.
              # Build: nix build .#checks.x86_64-linux.ci-dogfood-self-build-test --impure --option sandbox false
              ci-dogfood-self-build-test = import ./nix/tests/ci-dogfood-self-build.nix {
                inherit pkgs kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins-snix-build;
                aspenCliPackage = bins.full-aspen-cli-e2e;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gitRemoteAspenPackage = bins.full-git-remote-aspen;
                nixpkgsFlake = nixpkgs;
              };

              # Workspace build test: push 3-crate Cargo workspace to Forge, CI builds
              # with real external deps (serde, blake3, uhlc, thiserror) and path deps.
              # Proves workspace resolution, crates.io fetching, native code compilation.
              # Build: nix build .#checks.x86_64-linux.ci-dogfood-workspace-test --impure --option sandbox false
              ci-dogfood-workspace-test = import ./nix/tests/ci-dogfood-workspace.nix {
                inherit pkgs kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli-e2e;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gitRemoteAspenPackage = bins.full-git-remote-aspen;
                nixpkgsFlake = nixpkgs;
              };

              # Full workspace build test: push ALL 80 Aspen crates to Forge,
              # CI auto-triggers, NixBuildWorker compiles the entire aspen-node
              # binary (343K lines, 658 packages). The ultimate self-hosting proof.
              # Needs: 8GB RAM, 40GB disk, ~15-30 min build time inside the VM.
              # Build: nix build .#checks.x86_64-linux.ci-dogfood-full-workspace-test --impure --option sandbox false
              ci-dogfood-full-workspace-test = import ./nix/tests/ci-dogfood-full-workspace.nix {
                inherit pkgs fullSrc fullCargoVendorDir kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli-e2e;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gitRemoteAspenPackage = bins.full-git-remote-aspen;
                nixpkgsFlake = nixpkgs;
              };

              # HTTP proxy test: TCP tunnel and HTTP forward proxy over
              # iroh QUIC. Two nodes — server (aspen-node with proxy) and
              # client (aspen-cli proxy commands). Tests tunnel creation,
              # multiple/concurrent requests, forward proxy, and restart.
              # Build: nix build .#checks.x86_64-linux.proxy-tunnel-test
              proxy-tunnel-test = import ./nix/tests/proxy-tunnel.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node-proxy;
                aspenCliPackage = bins.full-aspen-cli-proxy;
              };

              # Multi-node proxy test: 3-node Raft cluster with proxy
              # enabled + external client. Tests TCP tunneling through
              # each node, cross-node tunneling, proxy during KV ops,
              # HTTP forward proxy, concurrent streams through multiple
              # nodes, proxy surviving leader failover, restarted node
              # proxy, large payload transfer, and multi-target tunnels.
              # Build: nix build .#checks.x86_64-linux.multi-node-proxy-test
              multi-node-proxy-test = import ./nix/tests/multi-node-proxy.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node-proxy;
                aspenCliPackage = bins.full-aspen-cli-proxy;
              };

              # Federation test: two independent single-node clusters
              # with separate identities. Tests federation status, trust
              # management, repo creation, and cluster independence.
              # Build: nix build .#checks.x86_64-linux.federation-test
              federation-test = import ./nix/tests/federation.nix {
                inherit pkgs kvPluginWasm forgePluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli-forge;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
              };

              # Net service mesh E2E: SOCKS5 proxy routing through a real Raft cluster.
              # Publishes an HTTP service, routes curl through SOCKS5 proxy over iroh QUIC tunnel.
              # Build: nix build .#checks.x86_64-linux.net-service-mesh-test --impure
              net-service-mesh-test = import ./nix/tests/net-service-mesh.nix {
                inherit pkgs kvPluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                aspenNetPackage = bins.full-aspen-net-daemon;
              };

              # Service mesh routing across Cloud Hypervisor microVMs.
              # Proves: Raft cluster → aspen-net SOCKS5 → iroh QUIC tunnel → socat → guest A HTTP
              # Build: nix build .#checks.x86_64-linux.microvm-net-mesh-test --impure
              microvm-net-mesh-test = import ./nix/tests/microvm-net-mesh.nix {
                inherit pkgs microvm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli;
                aspenNetPackage = bins.full-aspen-net-daemon;
              };

              # VirtioFS + Service Mesh combined: Guest A serves files from Raft KV via
              # VirtioFS mount, published to the mesh. Guest B reaches it through SOCKS5.
              # Proves: KV → VirtioFS → nginx → socat → TunnelAcceptor → iroh → SOCKS5 → curl
              # Build: nix build .#checks.x86_64-linux.microvm-virtiofs-net-test --impure
              microvm-virtiofs-net-test = import ./nix/tests/microvm-virtiofs-net.nix {
                inherit pkgs microvm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli;
                aspenNetPackage = bins.full-aspen-net-daemon;
                aspenClusterVirtiofsServer = self.packages.${system}.aspen-cluster-virtiofs-server;
              };

              # Multi-host service mesh: Two QEMU hosts, each with a CH microVM.
              # Guest A on Host A publishes HTTP service. Guest B on Host B reaches it
              # through SOCKS5 → iroh QUIC tunnel across QEMU virtual bridge.
              # Proves: cross-host iroh tunnel routing with real network separation.
              # Build: nix build .#checks.x86_64-linux.multihost-microvm-mesh-test --impure
              multihost-microvm-mesh-test = import ./nix/tests/multihost-microvm-mesh.nix {
                inherit pkgs microvm;
                aspenNodePackage = bins.full-aspen-node-plugins;
                aspenCliPackage = bins.full-aspen-cli;
                aspenNetPackage = bins.full-aspen-net-daemon;
              };
            }
            // lib.optionalAttrs (system == "x86_64-linux" && hasExternalRepos) {
              # snix gRPC bridge test: import files via snix-store → bridge → Aspen backends
              # Build: nix build .#checks.x86_64-linux.snix-bridge-test
              snix-bridge-test = import ./nix/tests/snix-bridge.nix {
                inherit pkgs;
                snixBridgePackage = bins.full-aspen-snix-bridge;
                snixStorePackage =
                  (import ./nix/snix-boot {
                    inherit lib pkgs snix-src crane;
                    rust-overlay = rust-overlay;
                  })
                  .snix-store;
              };
            }
            // lib.optionalAttrs (system == "x86_64-linux") {
              # Full-loop test: 3-stage pipeline orchestration (check → build → test).
              # Uses aspen-constants crate with 4 jobs: format-check (shell),
              # clippy-check (nix), build-and-test (nix), unit-tests (nix).
              # No WASM plugins required — uses ci-aspen-node (pure eval).
              # Build: nix build .#checks.x86_64-linux.ci-dogfood-full-loop-test --option sandbox false
              ci-dogfood-full-loop-test = import ./nix/tests/ci-dogfood-full-loop.nix {
                inherit pkgs;
                aspenNodePackage = ciVmTestBin {
                  name = "aspen-node";
                  features = ["ci" "docs" "hooks" "shell-worker" "automerge" "secrets" "git-bridge" "blob"];
                };
                aspenCliPackage = ciVmTestCliBin ["ci" "forge"];
                gitRemoteAspenPackage = bins.ci-git-remote-aspen;
                nixpkgsFlake = nixpkgs;
              };

              # Multi-node dogfood: 3-node cluster → Forge push → CI build →
              # rolling restart (follower-first) → quorum checks → re-election → KV persistence.
              # Build: nix build .#checks.x86_64-linux.rolling-restart-test --option sandbox false
              rolling-restart-test = import ./nix/tests/rolling-restart.nix {
                inherit pkgs;
                aspenNodePackage = ciVmTestBin {
                  name = "aspen-node";
                  features = ["docs" "hooks" "automerge" "blob"];
                };
                aspenCliPackage = ciVmTestCliBin [];
              };

              # Build: nix build .#checks.x86_64-linux.multi-node-dogfood-test --option sandbox false
              multi-node-dogfood-test = import ./nix/tests/multi-node-dogfood.nix {
                inherit pkgs;
                aspenNodePackage = ciVmTestBin {
                  name = "aspen-node";
                  features = ["ci" "docs" "hooks" "shell-worker" "automerge" "secrets" "git-bridge" "blob"];
                };
                aspenCliPackage = ciVmTestCliBin ["ci" "forge"];
                gitRemoteAspenPackage = bins.ci-git-remote-aspen;
                nixpkgsFlake = nixpkgs;
              };

              # SNIX microVM boot test: verifies the full snix boot chain
              # (snix-store virtiofs daemon → cloud-hypervisor → kernel +
              # initrd → snix-init → VirtioFS mount). Requires nested KVM.
              # Build: nix build .#checks.x86_64-linux.snix-boot-test --impure
              snix-boot-test = import ./nix/tests/snix-boot.nix {
                inherit pkgs;
                snixBoot = import ./nix/snix-boot {
                  inherit lib pkgs snix-src crane;
                  rust-overlay = rust-overlay;
                };
              };
            }
            // lib.optionalAttrs (system == "x86_64-linux" && hasExternalRepos) {
              # SNIX bridge + virtiofs round-trip: import a file via the bridge,
              # then boot a microVM that reads it back from /nix/store.
              # Requires nested KVM + external repos (snix-src).
              # Build: nix build .#checks.x86_64-linux.snix-bridge-virtiofs-test --impure
              snix-bridge-virtiofs-test = import ./nix/tests/snix-bridge-virtiofs.nix {
                inherit pkgs;
                snixBridgePackage = bins.full-aspen-snix-bridge;
                snixBoot = import ./nix/snix-boot {
                  inherit lib pkgs snix-src crane;
                  rust-overlay = rust-overlay;
                };
              };

              # snix nix-daemon protocol test: nix CLI talks to Aspen via Unix socket
              # Build: nix build .#checks.x86_64-linux.snix-daemon-test --impure
              snix-daemon-test = import ./nix/tests/snix-daemon.nix {
                inherit pkgs;
                snixBridgePackage = bins.full-aspen-snix-bridge;
              };

              # Native snix-build pipeline test: eval→drv→bubblewrap→PathInfoService→cache.
              # Proves in-process builds via snix-build work end-to-end in a real VM.
              # Build: nix build .#checks.x86_64-linux.snix-native-build-test --impure
              snix-native-build-test = import ./nix/tests/snix-native-build.nix {
                inherit pkgs kvPluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins-snix-build;
                aspenCliPackage = bins.full-aspen-cli-e2e;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gatewayPackage = bins.full-aspen-nix-cache-gateway;
              };

              # Fully-native npins build pipeline (zero subprocesses).
              # Uses snix-eval for .drvPath resolution instead of nix eval subprocess.
              # Build: nix build .#checks.x86_64-linux.npins-native-eval-test --impure
              npins-native-eval-test = import ./nix/tests/npins-native-eval.nix {
                inherit pkgs kvPluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins-snix-build;
                aspenCliPackage = bins.full-aspen-cli-e2e;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gatewayPackage = bins.full-aspen-nix-cache-gateway;
              };

              # Flake-compat native build (zero subprocesses for tarball inputs).
              # Uses embedded NixOS/flake-compat + snix-eval's fetchTarball for
              # in-process flake input resolution. No nix eval subprocess needed.
              # Build: nix build .#checks.x86_64-linux.snix-flake-native-build-test --impure
              snix-flake-native-build-test = import ./nix/tests/snix-flake-native-build.nix {
                inherit pkgs kvPluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins-snix-build;
                aspenCliPackage = bins.full-aspen-cli-e2e;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gatewayPackage = bins.full-aspen-nix-cache-gateway;
              };

              # Pure snix-build test: builds WITHOUT nix CLI in PATH.
              # Proves the native pipeline works when nix/nix-store are absent.
              # Build: nix build .#checks.x86_64-linux.snix-pure-build-test --impure
              snix-pure-build-test = import ./nix/tests/snix-pure-build-test.nix {
                inherit pkgs kvPluginWasm;
                aspenNodePackage = bins.full-aspen-node-plugins-snix-build;
                aspenCliPackage = bins.full-aspen-cli-e2e;
                aspenCliPlugins = bins.full-aspen-cli-plugins;
                gatewayPackage = bins.full-aspen-nix-cache-gateway;
              };

              # MicroVM smoke test: nginx in a Cloud Hypervisor microVM.
              # MicroVM + AspenFs VirtioFS integration test: nginx in a Cloud
              # Hypervisor guest serving files from AspenFs in-memory KV store.
              # Proves: AspenFs KV → VirtioFS daemon → vhost-user → CH → guest mount → nginx → curl
              # Build: nix build .#checks.x86_64-linux.microvm-nginx-test
              microvm-nginx-test = import ./nix/tests/microvm-nginx.nix {
                inherit pkgs microvm;
                inherit (self.packages.${system}) aspen-virtiofs-test-server;
              };

              # aspen-node in a Cloud Hypervisor microVM — proves the node boots,
              # initializes Raft consensus, and spawns the Iroh router inside a VM.
              # Build: nix build .#checks.x86_64-linux.microvm-aspen-node-test
              microvm-aspen-node-test = import ./nix/tests/microvm-aspen-node.nix {
                inherit pkgs microvm;
                inherit (self.packages.${system}) aspen-node-vm-test;
              };

              # 3-node Raft cluster + AspenFs VirtioFS connectivity test.
              # Proves: Raft consensus → cluster ticket exchange → VirtioFS daemon connects
              # Build: nix build .#checks.x86_64-linux.microvm-cluster-test
              microvm-cluster-test = import ./nix/tests/microvm-cluster.nix {
                inherit pkgs microvm;
                inherit (self.packages.${system}) aspen-node-vm-test aspen-fuse-vm-test;
              };

              # Full end-to-end: Raft cluster → VirtioFS → Cloud Hypervisor → nginx → HTTP
              # The ultimate integration test — data flows from Raft KV through every layer.
              # Build: nix build .#checks.x86_64-linux.microvm-raft-virtiofs-test
              microvm-raft-virtiofs-test = import ./nix/tests/microvm-raft-virtiofs.nix {
                inherit pkgs microvm;
                inherit (self.packages.${system}) aspen-node-vm-test aspen-cluster-virtiofs-server;
              };

              # Build: nix build .#checks.x86_64-linux.microvm-virtiofs-stress-test
              microvm-virtiofs-stress-test = import ./nix/tests/microvm-virtiofs-stress.nix {
                inherit pkgs microvm;
                inherit (self.packages.${system}) aspen-virtiofs-test-server;
              };

              # CI workspace pipeline: Raft cluster → AspenFs::with_prefix → VirtioFS → CH guest
              # Proves the CI executor's workspace data path with bidirectional I/O and namespace isolation.
              # Build: nix build .#checks.x86_64-linux.ci-workspace-virtiofs-test
              ci-workspace-virtiofs-test = import ./nix/tests/ci-workspace-virtiofs.nix {
                inherit pkgs microvm;
                inherit (self.packages.${system}) aspen-node-vm-test aspen-ci-workspace-server;
              };

              # VirtioFS snapshot/restore compatibility: snapshot with active VirtioFS → restore
              # → verify guest driver reconnects to fresh host-side daemons via vhost-user.
              # Tests overlayfs (tmpfs upper on virtiofs lower) survives snapshot/restore.
              # Build: nix build .#checks.x86_64-linux.vm-snapshot-virtiofs-test --impure
              vm-snapshot-virtiofs-test = import ./nix/tests/vm-snapshot-virtiofs.nix {
                inherit pkgs microvm;
                inherit (self.packages.${system}) aspen-virtiofs-test-server;
              };

              # End-to-end snapshot/restore CI pipeline: cluster → CH worker → golden
              # snapshot → CI job submission → restore from snapshot → job completion.
              # Also benchmarks cold-boot vs restore time and stress-tests 8 concurrent VMs.
              # Build: nix build .#checks.x86_64-linux.vm-snapshot-e2e-test --impure --option sandbox false
              vm-snapshot-e2e-test = let
                ciVmConfig = nixpkgs.lib.nixosSystem {
                  system = "x86_64-linux";
                  modules = [
                    microvm.nixosModules.microvm
                    (import ./nix/vms/ci-worker-node.nix {
                      inherit pkgs;
                      lib = nixpkgs.lib;
                      vmId = "aspen-ci-vm";
                      aspenNodePackage = self.packages.${system}.aspen-node-vm-test;
                    })
                  ];
                };
              in
                import ./nix/tests/vm-snapshot-e2e.nix {
                  inherit pkgs microvm;
                  inherit (self.packages.${system}) aspen-node-vm-test;
                  aspen-cli = bins.aspen-cli;
                  ciKernel = ciVmConfig.config.microvm.kernel;
                  ciInitrd = ciVmConfig.config.system.build.initialRamdisk;
                  ciToplevel = ciVmConfig.config.system.build.toplevel;
                };
            };

          # Base apps available on all systems
          apps = {
            aspen-node = flake-utils.lib.mkApp {
              drv = aspenNode;
              exePath = "/bin/aspen-node";
            };

            # aspen-tui extracted to ~/git/aspen-tui
            # aspen-cli extracted to ~/git/aspen-cli

            # aspen-cli app stubbed out (extracted to separate repo)
            aspen-cli = {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-cli-stub" ''
                echo "aspen-cli has been extracted to ~/git/aspen-cli"
                echo "Build it from that repository or use: nix run ~/git/aspen-cli"
                exit 1
              ''}";
            };

            # Verus formal verification (all specs: Core + Raft + Coordination + Cluster + Transport + Federation)
            # Usage: nix run .#verify-verus [command]
            # Commands: all (default), federation, core, cluster, ...
            # NOTE: most specs extracted to sibling repos; federation verifies in-repo
            verify-verus = {
              type = "app";
              program = "${pkgs.writeShellScript "verify-verus" ''
                set -e
                export LD_LIBRARY_PATH="${verusRustToolchain}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
                export VERUS_Z3_PATH="${verusRoot}/z3"
                export VERUS_ROOT="${verusRoot}"

                CMD="''${1:-all}"

                case "$CMD" in
                  all|"")
                    echo "=== Verus Formal Verification ==="
                    echo ""
                    echo "In-repo specs (can verify directly):"
                    echo "  Federation:   nix run .#verify-verus federation"
                    echo "  Commit DAG:   nix run .#verify-verus commit-dag"
                    echo ""
                    echo "Extracted to sibling repos:"
                    echo "  Core:         cd ~/git/aspen-core && verus crates/aspen-core/verus/lib.rs"
                    echo "  Cluster:      cd ~/git/aspen-cluster && verus crates/aspen-cluster/verus/lib.rs"
                    echo "  Raft:         cd ~/git/aspen-raft && nix run .#verify-verus"
                    echo "  Coordination: cd ~/git/aspen-coordination && nix run .#verify-verus"
                    echo "  Transport:    cd ~/git/aspen-transport && nix run .#verify-verus"
                    ;;
                  federation|fed)
                    echo "=== Verifying Federation specs ==="
                    "${verusRoot}/rust_verify" \
                      --crate-type=bin \
                      "''${FLAKE_ROOT:-.}/crates/aspen-federation/verus/lib.rs"
                    echo "Federation: all specs verified ✓"
                    ;;
                  commit-dag|commitdag)
                    echo "=== Verifying Commit DAG specs ==="
                    "${verusRoot}/rust_verify" \
                      --crate-type=lib \
                      "''${FLAKE_ROOT:-.}/crates/aspen-commit-dag/verus/lib.rs"
                    echo "Commit DAG: all specs verified ✓"
                    ;;
                  core)
                    echo "Core specs extracted to ~/git/aspen-core"
                    echo "Run: cd ~/git/aspen-core && verus crates/aspen-core/verus/lib.rs"
                    ;;
                  cluster)
                    echo "Cluster specs extracted to ~/git/aspen-cluster"
                    echo "Run: cd ~/git/aspen-cluster && verus crates/aspen-cluster/verus/lib.rs"
                    ;;
                  raft)
                    echo "Raft specs extracted to ~/git/aspen-raft"
                    echo "Run: cd ~/git/aspen-raft && nix run .#verify-verus"
                    ;;
                  coordination|coord)
                    echo "Coordination specs extracted to ~/git/aspen-coordination"
                    echo "Run: cd ~/git/aspen-coordination && nix run .#verify-verus"
                    ;;
                  transport)
                    echo "Transport specs extracted to ~/git/aspen-transport"
                    echo "Run: cd ~/git/aspen-transport && nix run .#verify-verus"
                    ;;
                  quick|check)
                    echo "All specs extracted to sibling repos. Run quick checks there."
                    ;;
                  help|--help|-h)
                    echo "Usage: nix run .#verify-verus [command]"
                    echo ""
                    echo "All verus specs have been extracted to sibling repos."
                    echo ""
                    echo "Commands:"
                    echo "  all (default)    Show all spec locations"
                    echo "  federation       Verify Federation specs (in-repo)"
                    echo "  core             Show Core repo location"
                    echo "  cluster          Show Cluster repo location"
                    echo "  raft             Show Raft repo location"
                    echo "  coordination     Show Coordination repo location"
                    echo "  transport        Show Transport repo location"
                    echo "  help             Show this help message"
                    ;;
                  *)
                    echo "Unknown command: $CMD"
                    echo "Run 'nix run .#verify-verus help' for usage"
                    exit 1
                    ;;
                esac
              ''}";
            };

            # Verus formal verification for coordination lock only
            # Usage: nix run .#verify-verus-coordination [command]
            # Commands: all (default), quick
            verify-verus-coordination = {
              type = "app";
              program = "${pkgs.writeShellScript "verify-verus-coordination" ''
                echo "Coordination verus specs extracted to ~/git/aspen-coordination"
                echo "Run: cd ~/git/aspen-coordination && nix run .#verify-verus"
              ''}";
            };

            # Verus formal verification for core primitives only
            # Usage: nix run .#verify-verus-core [command]
            # Commands: all (default), quick
            verify-verus-core = {
              type = "app";
              program = "${pkgs.writeShellScript "verify-verus-core" ''
                echo "Core verus specs extracted to ~/git/aspen-core"
                echo "Run: cd ~/git/aspen-core && verus crates/aspen-core/verus/lib.rs"
              ''}";
            };

            # Verus formal verification for cluster specs only
            # Usage: nix run .#verify-verus-cluster [command]
            # Commands: all (default), quick
            verify-verus-cluster = {
              type = "app";
              program = "${pkgs.writeShellScript "verify-verus-cluster" ''
                echo "Cluster verus specs extracted to ~/git/aspen-cluster"
                echo "Run: cd ~/git/aspen-cluster && verus crates/aspen-cluster/verus/lib.rs"
              ''}";
            };

            # Transport verus specs extracted to ~/git/aspen-transport
            verify-verus-transport = {
              type = "app";
              program = "${pkgs.writeShellScript "verify-verus-transport" ''
                echo "Transport verus specs extracted to ~/git/aspen-transport"
                echo "Run: cd ~/git/aspen-transport && nix run .#verify-verus"
              ''}";
            };

            # Verus inline ghost code check
            # Verifies that ghost code in production files compiles correctly
            # Usage: nix run .#verus-inline-check
            verus-inline-check = {
              type = "app";
              program = "${pkgs.writeShellScript "verus-inline-check" ''
                set -e
                echo "=== Verus Inline Ghost Code Check ==="
                echo "Checking that ghost code compiles with verus feature..."
                echo ""

                # Check aspen-raft compiles with verus feature
                echo "[1/4] Checking aspen-raft (verus feature disabled)..."
                ${rustToolChain}/bin/cargo check -p aspen-raft --quiet || {
                  echo "[FAIL] aspen-raft failed to compile without verus feature"
                  exit 1
                }
                echo "[PASS] aspen-raft compiles without verus feature"

                echo "[2/4] Checking aspen-raft (verus feature enabled)..."
                ${rustToolChain}/bin/cargo check -p aspen-raft --features verus --quiet || {
                  echo "[FAIL] aspen-raft failed to compile with verus feature"
                  exit 1
                }
                echo "[PASS] aspen-raft compiles with verus feature"

                # Check aspen-coordination compiles with verus feature
                echo "[3/4] Checking aspen-coordination (verus feature disabled)..."
                ${rustToolChain}/bin/cargo check -p aspen-coordination --quiet || {
                  echo "[FAIL] aspen-coordination failed to compile without verus feature"
                  exit 1
                }
                echo "[PASS] aspen-coordination compiles without verus feature"

                echo "[4/4] Checking aspen-coordination (verus feature enabled)..."
                ${rustToolChain}/bin/cargo check -p aspen-coordination --features verus --quiet || {
                  echo "[FAIL] aspen-coordination failed to compile with verus feature"
                  exit 1
                }
                echo "[PASS] aspen-coordination compiles with verus feature"

                echo ""
                echo "=== All inline ghost code checks passed ==="
              ''}";
            };

            # verify-verus-sync and verus-coverage: moved to ~/git/aspen-verus-metrics repo

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
                    aspenNode
                    # bins.aspen-cli  # Extracted to ~/git/aspen-cli
                    pkgs.bash
                    pkgs.coreutils
                    pkgs.curl
                    pkgs.netcat
                    pkgs.gnugrep
                    pkgs.gnused
                  ]
                }:$PATH"
                export ASPEN_NODE_BIN="${aspenNode}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                exec ${scriptsDir}/cluster.sh "$@"
              ''}";
            };

            # kitty-cluster removed (script deleted, aspen-tui extracted)

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

                exec ${aspenNode}/bin/aspen-node \
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
                    aspenNode
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

                export ASPEN_NODE_BIN="${aspenNode}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                export GIT_REMOTE_ASPEN_BIN="${bins.git-remote-aspen}/bin/git-remote-aspen"
                export ASPEN_NODE_COUNT="$NODE_COUNT"
                export ASPEN_VM_DIR="$VM_DIR"
                # Override PROJECT_DIR since script is in Nix store, not the actual project
                export PROJECT_DIR="$PWD"

                exec ${scriptsDir}/dogfood-vm.sh "$@"
              ''}";
            };

            # Local dogfood cluster (no VMs)
            # Usage: nix run .#dogfood-local
            # Runs nodes as local processes for quick testing.
            # Uses ci-aspen-node-snix-build for native in-process nix builds.
            dogfood-local = let
              dogfoodNode = bins.ci-aspen-node-snix-build;
              scriptsDir = pkgs.runCommand "aspen-scripts" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +w $out
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "dogfood-local" ''
                set -e

                export PATH="${
                  pkgs.lib.makeBinPath [
                    dogfoodNode
                    bins.aspen-cli
                    bins.git-remote-aspen
                    bins.ci-aspen-nix-cache-gateway
                    pkgs.bash
                    pkgs.coreutils
                    pkgs.gnugrep
                    pkgs.gnused
                    pkgs.git
                    pkgs.curl
                    pkgs.nix # Required for CI jobs that run nix build/fmt
                    pkgs.bubblewrap # Required for snix-build sandbox
                  ]
                }:$PATH"

                export ASPEN_NODE_BIN="${dogfoodNode}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                export GIT_REMOTE_ASPEN_BIN="${bins.git-remote-aspen}/bin/git-remote-aspen"
                export ASPEN_NIX_CACHE_GATEWAY_BIN="${bins.ci-aspen-nix-cache-gateway}/bin/aspen-nix-cache-gateway"
                export PROJECT_DIR="$PWD"

                exec ${scriptsDir}/dogfood-local.sh "$@"
              ''}";
            };

            # Local nodes with VM-isolated CI
            # Usage: nix run .#dogfood-local-vmci
            # Runs nodes as local processes, CI jobs in Cloud Hypervisor VMs
            # Prerequisites: sudo nix run .#setup-ci-network (network bridge + NAT)
            dogfood-local-vmci = let
              scriptsDir = pkgs.runCommand "aspen-scripts-vmci" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +w $out
              '';

              # Build the CI worker VM image (kernel + initrd + NixOS toplevel)
              # Pre-compute closure info for CI build dependencies.
              # This registers all build-time store paths (rust toolchain,
              # vendored crates, build artifacts) in the VM's nix database
              # so it can reuse them from the host's virtiofs-shared store
              # without needing network access to rebuild FODs.
              ciBuildClosureInfo = pkgs.closureInfo {
                rootPaths = [
                  ciCargoArtifacts
                  ciCargoVendorDir
                  # Include build outputs so the VM's nix-daemon recognizes
                  # all intermediate build products and final binaries.
                  self.checks.${system}.clippy
                  self.checks.${system}.build-node
                  self.checks.${system}.build-cli
                  self.checks.${system}.nextest-quick
                  self.checks.${system}.fmt
                ];
              };

              ciVmConfig = nixpkgs.lib.nixosSystem {
                system = "x86_64-linux";
                modules = [
                  microvm.nixosModules.microvm
                  (import ./nix/vms/ci-worker-node.nix {
                    inherit pkgs;
                    lib = nixpkgs.lib;
                    vmId = "aspen-ci-vm";
                    aspenNodePackage = bins.aspen-node-vmci;
                    inherit ciBuildClosureInfo;
                  })
                ];
              };
              ciKernel = ciVmConfig.config.microvm.kernel;
              ciInitrd = ciVmConfig.config.system.build.initialRamdisk;
              ciToplevel = ciVmConfig.config.system.build.toplevel;

              # Use the vmci-capable node (has ci-vm-executor feature)
              vmciNode = bins.aspen-node-vmci;
            in {
              type = "app";
              program = "${pkgs.writeShellScript "dogfood-local-vmci" ''
                set -e

                export PATH="${
                  pkgs.lib.makeBinPath [
                    vmciNode
                    bins.aspen-cli
                    bins.git-remote-aspen
                    pkgs.bash
                    pkgs.coreutils
                    pkgs.gnugrep
                    pkgs.gnused
                    pkgs.git
                    pkgs.nix
                    pkgs.cloud-hypervisor
                    pkgs.virtiofsd
                    pkgs.iproute2
                    pkgs.python3
                  ]
                }:$PATH"

                export ASPEN_NODE_BIN="${vmciNode}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                export GIT_REMOTE_ASPEN_BIN="${bins.git-remote-aspen}/bin/git-remote-aspen"
                export CLOUD_HYPERVISOR_BIN="${pkgs.cloud-hypervisor}/bin/cloud-hypervisor"
                export VIRTIOFSD_BIN="${pkgs.virtiofsd}/bin/virtiofsd"

                # CI worker VM image paths
                export ASPEN_CI_KERNEL_PATH="${ciKernel}/bzImage"
                export ASPEN_CI_INITRD_PATH="${ciInitrd}/initrd"
                export ASPEN_CI_TOPLEVEL_PATH="${ciToplevel}"

                export PROJECT_DIR="$PWD"

                exec ${scriptsDir}/dogfood-local-vmci.sh "$@"
              ''}";
            };

            # Network setup for CI VMs (run with sudo before dogfood-local-vmci)
            # Usage: sudo nix run .#setup-ci-network
            setup-ci-network = let
              scriptsDir = pkgs.runCommand "aspen-scripts-network" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +w $out
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "setup-ci-network" ''
                set -e
                export PATH="${
                  pkgs.lib.makeBinPath [
                    pkgs.iproute2
                    pkgs.nftables
                    pkgs.iptables
                    pkgs.coreutils
                  ]
                }:$PATH"
                exec ${scriptsDir}/setup-ci-network.sh "$@"
              ''}";
            };

            # build-plugins and deploy-plugin: moved to ~/git/aspen-plugins repo

            # Build plans are auto-generated via IFD (buildFromUnitGraphAuto).
            # No manual regeneration needed — plans update when Cargo.lock changes.
          };
        }
        // {
          # Packages exposed by the flake
          packages =
            {
              # ── Primary builds (unit2nix: per-crate, incremental) ─────
              default = aspenNode;
              aspen-node = aspenNode;
              aspen-cli = aspenCli;
              git-remote-aspen = aspenGitRemote;

              # ── Clippy (per-crate, only workspace members recompiled) ──
              clippy = u2nWorkspace.clippy.allWorkspaceMembers;

              # ── Crane builds (kept for specialized variants + VM tests)
              crane-aspen-node = bins.aspen-node;
              crane-aspen-cli = bins.aspen-cli;
              crane-git-remote-aspen = bins.git-remote-aspen;
              aspen-cli-forge = bins.aspen-cli-forge;
              # aspen-ci-agent extracted to ~/git/aspen-ci
              netwatch = netwatch;
              vm-test-setup = vm-test-setup;
              vm-test-run = vm-test-run;
              # Pre-built cargo dependencies for CI and binary cache.
              inherit cargoArtifacts nodeCargoArtifacts cliCargoArtifacts;
              # Verus formal verification tool (binary release with proper library setup)
              inherit verus;
              # Direct access to all Verus binaries (rust_verify, cargo-verus, z3, etc.)
              verus-root = verusRoot;
              # Z3 4.12.5 (required by Verus)
              z3 = z3_4_12_5;
            }
            // nixpkgs.lib.optionalAttrs (system == "x86_64-linux") (
              let
                # Build the CI worker VM configuration
                ciVmConfig = nixpkgs.lib.nixosSystem {
                  system = "x86_64-linux";
                  modules = [
                    microvm.nixosModules.microvm
                    (import ./nix/vms/ci-worker-node.nix {
                      inherit pkgs;
                      lib = nixpkgs.lib;
                      vmId = "aspen-ci-vm";
                      aspenNodePackage = aspenNode;
                    })
                  ];
                };
              in
                {
                  # CI VM kernel for Cloud Hypervisor worker
                  ci-vm-kernel = ciVmConfig.config.microvm.kernel;
                  # CI VM initrd for Cloud Hypervisor worker
                  ci-vm-initrd = ciVmConfig.config.system.build.initialRamdisk;
                  # CI VM toplevel (NixOS system with init script)
                  # The kernel cmdline needs init=${toplevel}/init to boot NixOS properly
                  ci-vm-toplevel = ciVmConfig.config.system.build.toplevel;
                  # Full CI VM runner (includes cloud-hypervisor command)
                  ci-vm-runner = ciVmConfig.config.microvm.runner.cloud-hypervisor;
                  # VirtioFS test initramfs
                  inherit virtiofsTestInitrd;

                  # ── Pure-eval workspace builds (no external repos needed) ─────
                  # Uses fullRawSrc (all workspace crates) with stubs for external
                  # deps (wasm plugins, iroh-proxy-utils, git deps). Works in pure
                  # eval mode — no --impure, no sibling repos required.
                  #
                  # Used for: VM integration test binaries (aspen-fuse, aspen-node)
                  # that need real workspace crates, not the lightweight stubs.
                }
                // (let
                  pureSrc = pkgs.runCommand "aspen-pure-src" {} ''
                    mkdir -p $out/aspen
                    cp -r ${fullRawSrc}/. $out/aspen/
                    chmod -R u+w $out/aspen

                    # Remove [patch.*] sections
                    if [ -f $out/aspen/.cargo/config.toml ]; then
                      ${pkgs.gnused}/bin/sed -i '/^\[patch\./,$d' $out/aspen/.cargo/config.toml
                    fi

                    # Stub aspen-wasm-plugin (workspace references it, but VM test binaries don't need it)
                    # Path matches workspace Cargo.toml: ../aspen-wasm-plugin/crates/aspen-wasm-plugin
                    mkdir -p "$out/aspen-wasm-plugin/crates/aspen-wasm-plugin/src"
                    cat > "$out/aspen-wasm-plugin/crates/aspen-wasm-plugin/Cargo.toml" << 'EOF'
                    [package]
                    name = "aspen-wasm-plugin"
                    version = "0.1.0"
                    edition = "2024"
                    [features]
                    default = []
                    hooks = []
                    EOF
                    echo '// stub' > "$out/aspen-wasm-plugin/crates/aspen-wasm-plugin/src/lib.rs"

                    # Stub aspen-dns (optional dep of aspen-net, dns feature disabled)
                    # Path from crates/aspen-net/: ../../../aspen-dns/crates/aspen-dns → $out/aspen-dns/crates/aspen-dns
                    mkdir -p "$out/aspen-dns/crates/aspen-dns/src"
                    cat > "$out/aspen-dns/crates/aspen-dns/Cargo.toml" << 'EOF'
                    [package]
                    name = "aspen-dns"
                    version = "0.1.0"
                    edition = "2024"
                    EOF
                    echo '// stub' > "$out/aspen-dns/crates/aspen-dns/src/lib.rs"

                    # Stub iroh-proxy-utils (referenced by aspen-proxy)
                    mkdir -p "$out/iroh-proxy-utils/src"
                    cat > "$out/iroh-proxy-utils/Cargo.toml" << 'EOF'
                    [package]
                    name = "iroh-proxy-utils"
                    version = "0.1.0"
                    edition = "2021"
                    EOF
                    echo '// stub' > "$out/iroh-proxy-utils/src/lib.rs"

                    # Stub git deps
                    stub() {
                      local name="$1"; shift
                      local dir="$out/aspen/.nix-stubs/$name"
                      mkdir -p "$dir/src"
                      {
                        echo '[package]'
                        echo "name = \"$name\""
                        echo 'version = "0.1.0"'
                        echo 'edition = "2024"'
                        echo '[features]'
                        for feat in "$@"; do echo "$feat = []"; done
                      } > "$dir/Cargo.toml"
                      echo '// stub' > "$dir/src/lib.rs"
                    }
                    stub mad-turmoil

                    # Rewrite non-snix git deps to path stubs in root Cargo.toml.
                    # snix deps (snix-castore, snix-store, nix-compat) stay as git deps
                    # so the cargo vendor dir (with overrideVendorGitCheckout) provides
                    # real snix source — needed by aspen-cache (nix-compat::narinfo).
                    ${pkgs.gnused}/bin/sed -i \
                      -e 's|mad-turmoil = { git = "[^"]*"[^}]*}|mad-turmoil = { path = ".nix-stubs/mad-turmoil", optional = true }|' \
                      $out/aspen/Cargo.toml
                    ${pkgs.gnused}/bin/sed -i '/dep:mad-turmoil/s/, "dep:mad-turmoil"//g' $out/aspen/Cargo.toml

                    # Strip git source lines from Cargo.lock for stubbed deps only.
                    # Keep snix.dev and tvlfyi (wu-manber, a snix dep) source lines —
                    # these are vendored via overrideVendorGitCheckout in pureCargoVendorDir.
                    ${pkgs.gnused}/bin/sed -i '/^source = "git+/{
                      /snix\.dev/b
                      /tvlfyi/b
                      d
                    }' $out/aspen/Cargo.lock

                    # Stub patchbay (test-only git dep)
                    stub patchbay

                    # Rewrite git deps in subcrates
                    find $out/aspen/crates -name Cargo.toml -exec ${pkgs.gnused}/bin/sed -i \
                      -e 's|mad-turmoil = { git = "[^"]*"[^}]*}|mad-turmoil = { path = "../../.nix-stubs/mad-turmoil", optional = true }|' \
                      -e 's|patchbay = { git = "[^"]*"[^}]*}|patchbay = { path = "../../.nix-stubs/patchbay" }|' \
                      -e 's|bolero = { version = "0.11"|bolero = { version = "0.13"|g' \
                      -e 's|bolero-generator = { version = "0.11"|bolero-generator = { version = "0.13"|g' \
                      -e 's|bolero = "0.11"|bolero = "0.13"|g' \
                      -e 's|bolero-generator = "0.11"|bolero-generator = "0.13"|g' \
                      {} \;
                  '';
                  pureBasicArgs =
                    basicArgs
                    // {
                      src = pureSrc;
                      postUnpack = ''sourceRoot="$sourceRoot/aspen"'';
                      cargoToml = ./Cargo.toml;
                      # Explicit cargoLock ensures buildDepsOnly includes the lockfile
                      # even when src changes invalidate the dummy source derivation.
                      cargoLock = pureSrc + "/aspen/Cargo.lock";
                      cargoExtraArgs = "";
                    };
                  pureCargoVendorDir = craneLib.vendorCargoDeps {
                    src = pureSrc + "/aspen";
                    overrideVendorGitCheckout = ps: drv: let
                      isSnixRepo =
                        builtins.any (
                          p:
                            builtins.isString (p.source or null)
                            && lib.hasPrefix "git+https://git.snix.dev/snix/snix.git" (p.source or "")
                        )
                        ps;
                    in
                      if isSnixRepo
                      then ensureGitCheckoutLock (drv.overrideAttrs (_old: {src = snix-src;}))
                      else ensureGitCheckoutLock drv;
                  };
                  pureBin = {
                    name,
                    cargoExtraArgs,
                  }:
                    craneLib.buildPackage (
                      pureBasicArgs
                      // {
                        inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                        inherit cargoExtraArgs;
                        cargoVendorDir = pureCargoVendorDir;
                        doCheck = false;
                      }
                    );
                in {
                  # AspenFs VirtioFS test server — serves in-memory KV over vhost-user socket
                  aspen-virtiofs-test-server = pureBin {
                    name = "aspen-virtiofs-test-server";
                    cargoExtraArgs = "--package aspen-fuse --bin aspen-virtiofs-test-server --features virtiofs";
                  };

                  # aspen-node for VM integration tests (no plugins, no wasm)
                  aspen-node-vm-test = pureBin {
                    name = "aspen-node-vm-test";
                    cargoExtraArgs = "--bin aspen-node --features ci,docs,hooks,shell-worker,automerge,secrets";
                  };

                  # aspen-fuse with VirtioFS for VM integration tests
                  aspen-fuse-vm-test = pureBin {
                    name = "aspen-fuse-vm-test";
                    cargoExtraArgs = "--package aspen-fuse --bin aspen-fuse --features virtiofs";
                  };

                  # VirtioFS server backed by a real Raft cluster (for integration tests)
                  aspen-cluster-virtiofs-server = pureBin {
                    name = "aspen-cluster-virtiofs-server";
                    cargoExtraArgs = "--package aspen-fuse --bin aspen-cluster-virtiofs-server --features virtiofs";
                  };

                  aspen-ci-workspace-server = pureBin {
                    name = "aspen-ci-workspace-server";
                    cargoExtraArgs = "--package aspen-fuse --bin aspen-ci-workspace-server --features virtiofs";
                  };

                  # aspen-node for serial dogfood VM (includes git-bridge for forge push)
                  aspen-node-serial-dogfood = pureBin {
                    name = "aspen-node-serial-dogfood";
                    cargoExtraArgs = "--bin aspen-node --features ci,docs,hooks,shell-worker,automerge,secrets,git-bridge,deploy";
                  };

                  # Full-featured node for physical deployments (Forge + CI + blob + all services)
                  aspen-node-clan = pureBin {
                    name = "aspen-node-clan";
                    cargoExtraArgs = "--bin aspen-node --features ci,forge,git-bridge,blob,docs,hooks,shell-worker,automerge,secrets,deploy";
                  };
                })
                // {
                  # Minimal nginx microVM — smoke test for the Cloud Hypervisor boot path
                  # Build: nix build .#nginx-vm-runner
                  # Run:   ./result/bin/microvm-run
                  nginx-vm-runner = let
                    nginxVm = nixpkgs.lib.nixosSystem {
                      system = "x86_64-linux";
                      modules = [
                        microvm.nixosModules.microvm
                        ./nix/vms/nginx-demo.nix
                      ];
                    };
                  in
                    nginxVm.config.microvm.runner.cloud-hypervisor;
                }
                // (let
                  snixBoot = import ./nix/snix-boot {
                    inherit lib pkgs snix-src crane;
                    rust-overlay = rust-overlay;
                  };
                in {
                  # ── SNIX Boot Components ──────────────────────────────
                  # Build: nix build .#snix-store
                  # Build: nix build .#snix-boot-runVM
                  # Run:   CH_CMDLINE=snix.find ./result/bin/run-snix-vm
                  snix-store = snixBoot.snix-store;
                  snix-boot-kernel = snixBoot.kernel;
                  snix-boot-initrd = snixBoot.initrd;
                  snix-boot-runVM = snixBoot.runVM;
                })
                // {
                  # Standalone NixOS VM for interactive dogfood testing via vm_boot + vm_serial.
                  # Build: nix build .#dogfood-serial-vm
                  # Boot:  vm_boot image="result/disk.qcow2" format="qcow2" memory="4096M"
                  dogfood-serial-vm = import ./nix/vms/serial-dogfood.nix {
                    inherit pkgs;
                    lib = nixpkgs.lib;
                    aspenNodePackage = self.packages.${system}.aspen-node-serial-dogfood;
                    aspenCliPackage = aspenCli;
                    gitRemoteAspenPackage = aspenGitRemote;
                    nixCacheGatewayPackage = bins.full-aspen-nix-cache-gateway;
                    nixpkgsFlake = nixpkgs;
                  };

                  # 3-node variant for testing cluster formation, deploy, and leadership transfer.
                  # Build: nix build .#dogfood-serial-multinode-vm
                  # Boot:  vm_boot image="result/disk.qcow2" format="qcow2" memory="6144M" cpus=4
                  dogfood-serial-multinode-vm = import ./nix/vms/serial-dogfood-multinode.nix {
                    inherit pkgs;
                    lib = nixpkgs.lib;
                    aspenNodePackage = self.packages.${system}.aspen-node-serial-dogfood;
                    aspenCliPackage = aspenCli;
                    gitRemoteAspenPackage = aspenGitRemote;
                    nixpkgsFlake = nixpkgs;
                  };
                }
            );
        }
        // {
          # Fuzzing development shell with nightly Rust
          # Usage: nix develop .#fuzz
          # Verus formal verification development shell
          devShells.verus = verusDevShell;

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
                # Verus formal verification tool
                verus
                # Z3 for Verus formal verification (bundled in verus, but also available standalone)
                z3_4_12_5
              ];

            env.RUST_SRC_PATH = "${rustToolChain}/lib/rustlib/src/rust/library";
            env.SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox-sandbox-shell}/bin/busybox";

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

            # Cloud Hypervisor binary for VirtioFS integration tests
            env.CLOUD_HYPERVISOR_BIN = "${pkgs.cloud-hypervisor}/bin/cloud-hypervisor";

            # VirtioFS test initramfs for cloud_hypervisor_virtiofs_test
            env.VIRTIOFS_TEST_INITRD = "${virtiofsTestInitrd}";

            shellHook = ''
              # Auto-stage scripts and decision docs so nix flake sees them
              if [ -d .git ]; then
                git add scripts/*.sh .claude/decisions/*.md 2>/dev/null || true
              fi

              # Add result/bin to PATH if it exists (from nix build)
              if [ -d "$PWD/result/bin" ]; then
                export PATH="$PWD/result/bin:$PATH"
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
        };
    }; # end of mkFlake
}
